use crate::memory::addr::{PhysAddr, PhysFrame, VirtAddr, PAGE_SIZE};
use crate::memory::pmm::Pmm;
use bitflags::bitflags;

bitflags! {
    /// x86_64 page table entry flags.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        const PRESENT       = 1 << 0;
        const WRITABLE      = 1 << 1;
        const USER          = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE      = 1 << 4;
        const ACCESSED      = 1 << 5;
        const DIRTY         = 1 << 6;
        const HUGE_PAGE     = 1 << 7;
        const GLOBAL        = 1 << 8;
        const NO_EXECUTE    = 1 << 63;
    }
}

/// A single page table entry (PTE).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub fn is_present(&self) -> bool {
        self.0 & PageFlags::PRESENT.bits() != 0
    }

    #[inline]
    pub fn flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub fn frame(&self) -> Option<PhysFrame> {
        if self.is_present() {
            Some(PhysFrame::containing_address(PhysAddr::new(
                self.0 & 0x000F_FFFF_FFFF_F000,
            )))
        } else {
            None
        }
    }

    #[inline]
    pub fn set(&mut self, frame: PhysFrame, flags: PageFlags) {
        self.0 = frame.start_address().as_u64() | flags.bits();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn raw(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn set_raw(&mut self, raw: u64) {
        self.0 = raw;
    }

    /// Get the physical frame address (bits 12-51).
    #[inline]
    pub fn frame_address(&self) -> u64 {
        self.0 & 0x000F_FFFF_FFFF_F000
    }

    /// Create a new entry from a physical address and raw flags.
    #[inline]
    pub fn new(phys_addr: u64, flags: u64) -> Self {
        Self((phys_addr & 0x000F_FFFF_FFFF_F000) | (flags & 0xFFF))
    }
}

/// A page table: 512 entries, 4 KiB aligned.
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn empty() -> Self {
        const EMPTY: PageTableEntry = PageTableEntry::empty();
        Self {
            entries: [EMPTY; 512],
        }
    }

    /// Zero out all entries.
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }
}

/// Page mapper that can create and modify page table mappings.
///
/// Uses the HHDM to access page table frames.
pub struct PageMapper {
    p4_frame: PhysFrame,
    hhdm_offset: u64,
}

impl PageMapper {
    /// Create a mapper for the given PML4 frame.
    pub fn new(p4_frame: PhysFrame, hhdm_offset: u64) -> Self {
        Self {
            p4_frame,
            hhdm_offset,
        }
    }

    /// Create a mapper for the currently active PML4 (read from CR3).
    pub fn current(hhdm_offset: u64) -> Self {
        let cr3: u64;
        // SAFETY: Reading CR3 is always safe.
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
        }
        let p4_frame = PhysFrame::containing_address(PhysAddr::new(cr3 & 0x000F_FFFF_FFFF_F000));
        Self::new(p4_frame, hhdm_offset)
    }

    /// Map a virtual page to a physical frame with the given flags.
    ///
    /// Allocates intermediate page table frames from the PMM as needed.
    /// Returns an error if the page is already mapped or if frame allocation fails.
    pub fn map(
        &mut self,
        virt: VirtAddr,
        frame: PhysFrame,
        flags: PageFlags,
    ) -> Result<(), MapError> {
        let pmm = Pmm::get();

        // SAFETY: All page table pointers are obtained via HHDM from valid
        // physical frames. We hold &mut self so no concurrent modification.
        unsafe {
            let p4 = self.p4_table();
            let p3_ptr = Self::next_table_create(
                &mut (*p4).entries[virt.p4_index()],
                pmm,
                self.hhdm_offset,
            )?;
            let p2_ptr = Self::next_table_create(
                &mut (*p3_ptr).entries[virt.p3_index()],
                pmm,
                self.hhdm_offset,
            )?;
            let p1_ptr = Self::next_table_create(
                &mut (*p2_ptr).entries[virt.p2_index()],
                pmm,
                self.hhdm_offset,
            )?;

            let entry = &mut (*p1_ptr).entries[virt.p1_index()];
            if entry.is_present() {
                return Err(MapError::AlreadyMapped);
            }

            entry.set(frame, flags | PageFlags::PRESENT);
        }
        Ok(())
    }

    /// Unmap a virtual page. Returns the previously mapped frame.
    pub fn unmap(&mut self, virt: VirtAddr) -> Result<PhysFrame, MapError> {
        // SAFETY: All page table pointers are obtained via HHDM from valid
        // physical frames. We hold &mut self so no concurrent modification.
        unsafe {
            let p4 = self.p4_table();
            let p3_ptr = Self::next_table(&(*p4).entries[virt.p4_index()], self.hhdm_offset)
                .ok_or(MapError::NotMapped)?;
            let p2_ptr = Self::next_table(&(*p3_ptr).entries[virt.p3_index()], self.hhdm_offset)
                .ok_or(MapError::NotMapped)?;
            let p1_ptr = Self::next_table(&(*p2_ptr).entries[virt.p2_index()], self.hhdm_offset)
                .ok_or(MapError::NotMapped)?;

            let entry = &mut (*p1_ptr).entries[virt.p1_index()];
            let frame = entry.frame().ok_or(MapError::NotMapped)?;
            entry.clear();

            tlb_flush_page(virt);

            Ok(frame)
        }
    }

    /// Translate a virtual address to its mapped physical address.
    pub fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        // SAFETY: All page table pointers are obtained via HHDM from valid
        // physical frames. We hold &self so page tables are not being modified.
        unsafe {
            let p4 = self.p4_table();
            let p3_ptr = Self::next_table(&(*p4).entries[virt.p4_index()], self.hhdm_offset)?;
            let p2_ptr = Self::next_table(&(*p3_ptr).entries[virt.p3_index()], self.hhdm_offset)?;
            let p1_ptr = Self::next_table(&(*p2_ptr).entries[virt.p2_index()], self.hhdm_offset)?;

            let entry = &(*p1_ptr).entries[virt.p1_index()];
            let frame = entry.frame()?;
            Some(frame.start_address() + virt.page_offset())
        }
    }

    /// Map a range of pages. On failure, unmaps any pages that were mapped.
    pub fn map_range(
        &mut self,
        virt_start: VirtAddr,
        phys_start: PhysFrame,
        page_count: u64,
        flags: PageFlags,
    ) -> Result<(), MapError> {
        for i in 0..page_count {
            let virt = VirtAddr::new_canonicalize(virt_start.as_u64() + i * PAGE_SIZE);
            let frame = PhysFrame::containing_address(PhysAddr::new(
                phys_start.start_address().as_u64() + i * PAGE_SIZE,
            ));
            if let Err(e) = self.map(virt, frame, flags) {
                // Rollback: unmap pages we already mapped
                for j in 0..i {
                    let rollback_virt =
                        VirtAddr::new_canonicalize(virt_start.as_u64() + j * PAGE_SIZE);
                    let _ = self.unmap(rollback_virt);
                }
                return Err(e);
            }
        }
        Ok(())
    }

    /// Get the PML4 frame.
    pub fn p4_frame(&self) -> PhysFrame {
        self.p4_frame
    }

    fn p4_table(&self) -> *mut PageTable {
        let virt = self.p4_frame.start_address().as_u64() + self.hhdm_offset;
        virt as *mut PageTable
    }

    fn next_table(entry: &PageTableEntry, hhdm_offset: u64) -> Option<*mut PageTable> {
        if !entry.is_present() {
            return None;
        }
        let frame = entry.frame()?;
        let virt = frame.start_address().as_u64() + hhdm_offset;
        Some(virt as *mut PageTable)
    }

    fn next_table_create(
        entry: &mut PageTableEntry,
        pmm: &Pmm,
        hhdm_offset: u64,
    ) -> Result<*mut PageTable, MapError> {
        if !entry.is_present() {
            let frame = pmm.alloc().ok_or(MapError::OutOfMemory)?;
            // Frame is already zeroed by PMM
            entry.set(
                frame,
                PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::USER,
            );
        }
        let frame = entry.frame().unwrap();
        let virt = frame.start_address().as_u64() + hhdm_offset;
        Ok(virt as *mut PageTable)
    }
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    NotMapped,
    OutOfMemory,
}

/// Flush TLB for a single page.
#[inline]
pub fn tlb_flush_page(addr: VirtAddr) {
    // SAFETY: INVLPG is always safe to call — it only invalidates cached translations.
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr.as_u64(), options(nostack, preserves_flags));
    }
}

/// Flush the entire TLB by reloading CR3.
#[inline]
pub fn tlb_flush_all() {
    // SAFETY: Reloading CR3 with the same value flushes all non-global TLB entries.
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack));
    }
}
