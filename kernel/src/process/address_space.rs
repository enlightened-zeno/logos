extern crate alloc;

use crate::memory::addr::{PhysAddr, PhysFrame, VirtAddr, PAGE_SIZE};
use crate::memory::paging::{PageFlags, PageMapper, PageTable, PageTableEntry};
use crate::memory::pmm::Pmm;
use crate::syscall::errno::Errno;

/// User address space layout.
pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FFFF_F000;
pub const USER_STACK_SIZE: u64 = 64 * PAGE_SIZE; // 256 KiB
pub const USER_STACK_BOTTOM: u64 = USER_STACK_TOP - USER_STACK_SIZE;
pub const USER_HEAP_START: u64 = 0x0000_4000_0000_0000;

/// A user-mode address space.
pub struct AddressSpace {
    /// PML4 physical frame.
    pub pml4_frame: PhysFrame,
    /// Current program break (heap end).
    pub brk: u64,
    /// HHDM offset for page table access.
    pub hhdm_offset: u64,
}

impl AddressSpace {
    /// Create a new user address space with kernel mappings cloned.
    pub fn new(hhdm_offset: u64) -> Result<Self, Errno> {
        let pmm = Pmm::get();

        // Allocate a new PML4 frame
        let pml4_frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
        let pml4_virt = (pml4_frame.start_address().as_u64() + hhdm_offset) as *mut PageTable;

        // Zero the new PML4
        // SAFETY: Frame is freshly allocated and mapped via HHDM.
        unsafe {
            (*pml4_virt).zero();
        }

        // Copy the kernel half (entries 256-511) from the current PML4
        let current_cr3: u64;
        // SAFETY: Reading CR3 is always safe.
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) current_cr3, options(nomem, nostack));
        }
        let current_pml4_phys = current_cr3 & 0x000F_FFFF_FFFF_F000;
        let current_pml4 = (current_pml4_phys + hhdm_offset) as *const PageTable;

        // SAFETY: Both PML4s are valid page tables mapped via HHDM.
        unsafe {
            for i in 256..512 {
                (*pml4_virt).entries[i] = (*current_pml4).entries[i];
            }
        }

        Ok(Self {
            pml4_frame,
            brk: USER_HEAP_START,
            hhdm_offset,
        })
    }

    /// Map a page in this address space.
    pub fn map_page(
        &self,
        virt: VirtAddr,
        frame: PhysFrame,
        flags: PageFlags,
    ) -> Result<(), Errno> {
        let mut mapper = PageMapper::new(self.pml4_frame, self.hhdm_offset);
        mapper.map(virt, frame, flags).map_err(|_| Errno::ENOMEM)
    }

    /// Allocate and map a page in this address space.
    pub fn alloc_and_map(&self, virt: VirtAddr, flags: PageFlags) -> Result<PhysFrame, Errno> {
        let pmm = Pmm::get();
        let frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
        self.map_page(virt, frame, flags)?;
        Ok(frame)
    }

    /// Map the user stack region.
    pub fn map_user_stack(&self) -> Result<(), Errno> {
        let pages = USER_STACK_SIZE / PAGE_SIZE;
        for i in 0..pages {
            let vaddr = VirtAddr::new(USER_STACK_BOTTOM + i * PAGE_SIZE);
            self.alloc_and_map(
                vaddr,
                PageFlags::WRITABLE | PageFlags::USER | PageFlags::NO_EXECUTE,
            )?;
        }
        Ok(())
    }

    /// Load an ELF segment into this address space.
    pub fn load_segment(
        &self,
        vaddr: u64,
        data: &[u8],
        memsz: u64,
        flags: PageFlags,
    ) -> Result<(), Errno> {
        let page_start = vaddr & !(PAGE_SIZE - 1);
        let page_end = (vaddr + memsz).div_ceil(PAGE_SIZE) * PAGE_SIZE;
        let num_pages = (page_end - page_start) / PAGE_SIZE;

        // Allocate and map pages
        for i in 0..num_pages {
            let page_vaddr = VirtAddr::new(page_start + i * PAGE_SIZE);
            self.alloc_and_map(page_vaddr, flags | PageFlags::USER)?;
        }

        // Copy data into the mapped pages via HHDM
        let mapper = PageMapper::new(self.pml4_frame, self.hhdm_offset);
        for (i, &byte) in data.iter().enumerate() {
            let target_vaddr = vaddr + i as u64;
            if let Some(phys) = mapper.translate(VirtAddr::new(target_vaddr)) {
                let dest = (phys.as_u64() + self.hhdm_offset) as *mut u8;
                // SAFETY: Physical address is valid and mapped via HHDM.
                unsafe { *dest = byte };
            }
        }

        Ok(())
    }

    /// Get the PML4 physical address (for CR3).
    pub fn cr3(&self) -> u64 {
        self.pml4_frame.start_address().as_u64()
    }

    /// Fork this address space (COW clone).
    ///
    /// Creates a new address space that shares all user pages with the parent.
    /// All shared pages are marked read-only in both parent and child. A write
    /// fault on a shared page triggers a copy (handled by the page fault handler).
    pub fn fork(&self) -> Result<Self, Errno> {
        use crate::memory::cow;
        use crate::memory::paging::PageTableEntry;

        let pmm = Pmm::get();

        // Create new PML4
        let child_pml4_frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
        let child_pml4_virt =
            (child_pml4_frame.start_address().as_u64() + self.hhdm_offset) as *mut PageTable;

        // SAFETY: Frame is freshly allocated and mapped via HHDM.
        unsafe { (*child_pml4_virt).zero() };

        let parent_pml4_virt =
            (self.pml4_frame.start_address().as_u64() + self.hhdm_offset) as *const PageTable;

        // SAFETY: Both PML4s are valid and mapped via HHDM.
        unsafe {
            // Copy kernel half (entries 256-511) — shared, not COW
            for i in 256..512 {
                (*child_pml4_virt).entries[i] = (*parent_pml4_virt).entries[i];
            }

            // Clone user half (entries 0-255) with COW
            for p4_idx in 0..256 {
                let p4_entry = &(*parent_pml4_virt).entries[p4_idx];
                if !p4_entry.is_present() {
                    continue;
                }

                // Allocate new PDP for child
                let child_pdp_frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
                let child_pdp =
                    (child_pdp_frame.start_address().as_u64() + self.hhdm_offset) as *mut PageTable;
                (*child_pdp).zero();

                let parent_pdp = Self::entry_to_table(p4_entry, self.hhdm_offset);

                for p3_idx in 0..512 {
                    let p3_entry = &(*parent_pdp).entries[p3_idx];
                    if !p3_entry.is_present() {
                        continue;
                    }

                    // Allocate new PD for child
                    let child_pd_frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
                    let child_pd = (child_pd_frame.start_address().as_u64() + self.hhdm_offset)
                        as *mut PageTable;
                    (*child_pd).zero();

                    let parent_pd = Self::entry_to_table(p3_entry, self.hhdm_offset);

                    for p2_idx in 0..512 {
                        let p2_entry = &(*parent_pd).entries[p2_idx];
                        if !p2_entry.is_present() {
                            continue;
                        }

                        // Allocate new PT for child
                        let child_pt_frame = pmm.alloc().ok_or(Errno::ENOMEM)?;
                        let child_pt = (child_pt_frame.start_address().as_u64() + self.hhdm_offset)
                            as *mut PageTable;
                        (*child_pt).zero();

                        let parent_pt = Self::entry_to_table(p2_entry, self.hhdm_offset);

                        for p1_idx in 0..512 {
                            let parent_pte =
                                &mut (*parent_pt).entries[p1_idx] as *mut PageTableEntry;
                            let pte = &*parent_pte;
                            if !pte.is_present() {
                                continue;
                            }

                            // Get the physical frame this PTE points to
                            let frame_addr = pte.frame_address();
                            let frame = PhysFrame::containing_address(PhysAddr::new(frame_addr));

                            // Mark parent page as read-only (remove WRITABLE)
                            let mut flags = pte.raw() & !PageFlags::WRITABLE.bits();
                            // Set a COW marker bit (use bit 9, available for OS use)
                            flags |= 1 << 9; // COW bit
                            (*parent_pte).set_raw(flags);

                            // Child gets same mapping, also read-only with COW bit
                            (*child_pt).entries[p1_idx].set_raw(flags);

                            // Increment COW reference count
                            cow::inc_ref(frame);
                        }

                        // Wire child PT into child PD
                        (*child_pd).entries[p2_idx] = PageTableEntry::new(
                            child_pt_frame.start_address().as_u64(),
                            p2_entry.raw() & 0xFFF, // Preserve flags
                        );
                    }

                    // Wire child PD into child PDP
                    (*child_pdp).entries[p3_idx] = PageTableEntry::new(
                        child_pd_frame.start_address().as_u64(),
                        p3_entry.raw() & 0xFFF,
                    );
                }

                // Wire child PDP into child PML4
                (*child_pml4_virt).entries[p4_idx] = PageTableEntry::new(
                    child_pdp_frame.start_address().as_u64(),
                    p4_entry.raw() & 0xFFF,
                );
            }
        }

        // Flush TLB for parent (pages are now read-only)
        // SAFETY: Flushing TLB is always safe.
        unsafe {
            core::arch::asm!("mov rax, cr3", "mov cr3, rax", out("rax") _, options(nostack));
        }

        Ok(Self {
            pml4_frame: child_pml4_frame,
            brk: self.brk,
            hhdm_offset: self.hhdm_offset,
        })
    }

    /// Convert a page table entry to a table pointer via HHDM.
    unsafe fn entry_to_table(entry: &PageTableEntry, hhdm_offset: u64) -> *mut PageTable {
        let phys = entry.frame_address();
        (phys + hhdm_offset) as *mut PageTable
    }
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // TODO: Walk and free all user page table frames and mapped frames.
        // For now, we leak them — this will be fixed with proper process cleanup.
    }
}
