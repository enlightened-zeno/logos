extern crate alloc;

use crate::memory::addr::{PhysFrame, VirtAddr, PAGE_SIZE};
use crate::memory::paging::{PageFlags, PageMapper, PageTable};
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
    hhdm_offset: u64,
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
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // TODO: Walk and free all user page table frames and mapped frames.
        // For now, we leak them — this will be fixed with proper process cleanup.
    }
}
