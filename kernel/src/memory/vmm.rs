use crate::memory::addr::{PhysAddr, PhysFrame, VirtAddr, PAGE_SIZE};
use crate::memory::paging::{PageFlags, PageMapper};
use crate::memory::pmm::Pmm;
use spin::Once;

/// Kernel virtual address space layout.
pub mod layout {
    use crate::memory::addr::VirtAddr;

    /// HHDM direct map: the bootloader maps all physical memory here.
    /// Actual base is provided by Limine at boot.
    pub static mut PHYS_MEM_OFFSET: u64 = 0;

    /// Kernel heap starts at a fixed virtual address.
    pub const KERNEL_HEAP_START: VirtAddr = VirtAddr::new_unchecked(0xFFFF_C000_0000_0000);
    /// Initial kernel heap size: 2 MiB
    pub const KERNEL_HEAP_INITIAL_SIZE: u64 = 2 * 1024 * 1024;
    /// Maximum kernel heap size: 256 MiB
    pub const KERNEL_HEAP_MAX_SIZE: u64 = 256 * 1024 * 1024;
}

/// Kernel Virtual Memory Manager.
pub struct Vmm {
    mapper: spin::Mutex<PageMapper>,
}

static VMM: Once<Vmm> = Once::new();

impl Vmm {
    /// Initialize the VMM using the current page tables set up by the bootloader.
    ///
    /// # Safety
    /// Must be called once during single-threaded boot after PMM is initialized.
    pub unsafe fn init(hhdm_offset: u64) {
        // SAFETY: Single-threaded boot, only written once.
        unsafe {
            layout::PHYS_MEM_OFFSET = hhdm_offset;
        }

        let mapper = PageMapper::current(hhdm_offset);

        VMM.call_once(|| Vmm {
            mapper: spin::Mutex::new(mapper),
        });
    }

    pub fn get() -> &'static Vmm {
        VMM.get().expect("VMM not initialized")
    }

    /// Map a virtual page to a physical frame with specified flags.
    pub fn map_page(
        &self,
        virt: VirtAddr,
        frame: PhysFrame,
        flags: PageFlags,
    ) -> Result<(), crate::memory::paging::MapError> {
        self.mapper.lock().map(virt, frame, flags)
    }

    /// Unmap a virtual page and return the physical frame.
    pub fn unmap_page(&self, virt: VirtAddr) -> Result<PhysFrame, crate::memory::paging::MapError> {
        self.mapper.lock().unmap(virt)
    }

    /// Translate a virtual address to a physical address.
    pub fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        self.mapper.lock().translate(virt)
    }

    /// Allocate and map `page_count` pages at the given virtual address.
    ///
    /// Allocates physical frames from the PMM and maps them with the given flags.
    /// On failure, frees all allocated frames and unmaps partial work.
    pub fn alloc_and_map(
        &self,
        virt_start: VirtAddr,
        page_count: u64,
        flags: PageFlags,
    ) -> Result<(), crate::memory::paging::MapError> {
        let pmm = Pmm::get();
        let mut mapper = self.mapper.lock();

        for i in 0..page_count {
            let virt = VirtAddr::new_canonicalize(virt_start.as_u64() + i * PAGE_SIZE);
            let frame = pmm
                .alloc()
                .ok_or(crate::memory::paging::MapError::OutOfMemory)?;

            if let Err(e) = mapper.map(virt, frame, flags) {
                // Rollback: free this frame and unmap/free previous ones
                // SAFETY: Frame was just allocated and not yet used.
                unsafe { pmm.dealloc(frame) };
                for j in 0..i {
                    let rollback_virt =
                        VirtAddr::new_canonicalize(virt_start.as_u64() + j * PAGE_SIZE);
                    if let Ok(f) = mapper.unmap(rollback_virt) {
                        // SAFETY: Frame was allocated during this operation.
                        unsafe { pmm.dealloc(f) };
                    }
                }
                return Err(e);
            }
        }
        Ok(())
    }

    /// Free and unmap `page_count` pages starting at the given virtual address.
    pub fn unmap_and_free(&self, virt_start: VirtAddr, page_count: u64) {
        let pmm = Pmm::get();
        let mut mapper = self.mapper.lock();

        for i in 0..page_count {
            let virt = VirtAddr::new_canonicalize(virt_start.as_u64() + i * PAGE_SIZE);
            if let Ok(frame) = mapper.unmap(virt) {
                // SAFETY: Frame was previously allocated and is no longer mapped.
                unsafe { pmm.dealloc(frame) };
            }
        }
    }

    /// Get the PML4 physical frame for this address space.
    pub fn p4_frame(&self) -> PhysFrame {
        self.mapper.lock().p4_frame()
    }
}
