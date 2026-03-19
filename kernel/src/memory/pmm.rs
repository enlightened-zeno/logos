use crate::memory::addr::{PhysAddr, PhysFrame, PAGE_SIZE};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, Once};

/// Memory zones matching x86_64 DMA constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// 0..16 MiB — legacy ISA DMA
    Dma16,
    /// 16 MiB..4 GiB — 32-bit DMA
    Dma32,
    /// 4 GiB+ — general purpose
    Normal,
}

impl Zone {
    pub const COUNT: usize = 3;

    pub fn for_address(addr: PhysAddr) -> Self {
        let a = addr.as_u64();
        if a < 0x100_0000 {
            Zone::Dma16
        } else if a < 0x1_0000_0000 {
            Zone::Dma32
        } else {
            Zone::Normal
        }
    }

    fn index(self) -> usize {
        match self {
            Zone::Dma16 => 0,
            Zone::Dma32 => 1,
            Zone::Normal => 2,
        }
    }
}

/// Node stored in-place in each free frame (intrusive free list).
/// When a frame is free, its first 8 bytes hold the next pointer.
#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

/// Per-zone free list state.
struct ZoneFreeList {
    head: *mut FreeNode,
    count: u64,
}

// SAFETY: We only access these through Mutex.
unsafe impl Send for ZoneFreeList {}

/// Physical Memory Manager.
///
/// Uses per-zone intrusive free lists. Each free frame stores a pointer to
/// the next free frame at its virtual address (via the HHDM direct map).
pub struct Pmm {
    zones: [Mutex<ZoneFreeList>; Zone::COUNT],
    total_frames: AtomicU64,
    free_frames: AtomicU64,
    hhdm_offset: u64,
}

/// Global PMM instance. Initialized once during boot.
static PMM: Once<Pmm> = Once::new();

impl Pmm {
    /// Initialize the PMM with the given HHDM offset and memory map.
    ///
    /// # Safety
    /// - Must be called exactly once during single-threaded boot.
    /// - `hhdm_offset` must be the correct higher-half direct map base.
    /// - Memory map entries must be accurate.
    pub unsafe fn init(hhdm_offset: u64, memory_map: &[MemoryRegion]) {
        PMM.call_once(|| Pmm {
            zones: [
                Mutex::new(ZoneFreeList {
                    head: core::ptr::null_mut(),
                    count: 0,
                }),
                Mutex::new(ZoneFreeList {
                    head: core::ptr::null_mut(),
                    count: 0,
                }),
                Mutex::new(ZoneFreeList {
                    head: core::ptr::null_mut(),
                    count: 0,
                }),
            ],
            total_frames: AtomicU64::new(0),
            free_frames: AtomicU64::new(0),
            hhdm_offset,
        });

        let pmm = Self::get();

        let mut total = 0u64;
        for region in memory_map {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }

            // Align region start up and end down to page boundaries
            let start = PhysAddr::new(region.base).align_up(PAGE_SIZE).as_u64();
            let end = PhysAddr::new(region.base + region.length)
                .align_down(PAGE_SIZE)
                .as_u64();

            if start >= end {
                continue;
            }

            let mut addr = start;
            while addr < end {
                // Skip the first 1 MiB — reserved for legacy hardware, BIOS, etc.
                if addr < 0x10_0000 {
                    addr += PAGE_SIZE;
                    continue;
                }

                let phys = PhysAddr::new(addr);
                let frame = phys.containing_frame();
                let zone = Zone::for_address(phys);

                // Zero the frame via HHDM mapping
                let virt_ptr = (addr + pmm.hhdm_offset) as *mut u8;
                // SAFETY: Frame is in usable memory and HHDM provides a valid mapping.
                unsafe {
                    core::ptr::write_bytes(virt_ptr, 0, PAGE_SIZE as usize);
                }

                pmm.push_frame(zone, frame);
                total += 1;
                addr += PAGE_SIZE;
            }
        }

        pmm.total_frames.store(total, Ordering::Relaxed);
        pmm.free_frames.store(total, Ordering::Relaxed);
    }

    /// Get the global PMM instance.
    ///
    /// # Panics
    /// Panics if PMM has not been initialized.
    pub fn get() -> &'static Pmm {
        PMM.get().expect("PMM not initialized")
    }

    /// Allocate a single physical frame, preferring the given zone.
    /// Falls back to higher zones if the preferred zone is empty.
    /// Returns `None` if all zones are exhausted.
    pub fn alloc_frame(&self, preferred_zone: Zone) -> Option<PhysFrame> {
        // Try preferred zone first, then fall back to higher zones
        let order = match preferred_zone {
            Zone::Dma16 => &[Zone::Dma16, Zone::Dma32, Zone::Normal],
            Zone::Dma32 => &[Zone::Dma32, Zone::Normal, Zone::Dma16],
            Zone::Normal => &[Zone::Normal, Zone::Dma32, Zone::Dma16],
        };

        for &zone in order {
            if let Some(frame) = self.pop_frame(zone) {
                self.free_frames.fetch_sub(1, Ordering::Relaxed);
                // Zero the frame before returning
                let virt_ptr = (frame.start_address().as_u64() + self.hhdm_offset) as *mut u8;
                // SAFETY: Frame was just allocated and HHDM provides valid mapping.
                unsafe {
                    core::ptr::write_bytes(virt_ptr, 0, PAGE_SIZE as usize);
                }
                return Some(frame);
            }
        }

        None
    }

    /// Allocate from the Normal zone (most common case).
    pub fn alloc(&self) -> Option<PhysFrame> {
        if crate::fault::should_fail(crate::fault::InjectionPoint::PmmAlloc) {
            return None;
        }
        self.alloc_frame(Zone::Normal)
    }

    /// Free a physical frame back to its appropriate zone.
    ///
    /// # Safety
    /// - The frame must have been allocated by this PMM.
    /// - The frame must not be currently in use.
    /// - The frame must not be freed more than once.
    pub unsafe fn dealloc(&self, frame: PhysFrame) {
        let zone = Zone::for_address(frame.start_address());
        self.push_frame(zone, frame);
        self.free_frames.fetch_add(1, Ordering::Relaxed);
    }

    /// Total number of managed frames.
    pub fn total_frames(&self) -> u64 {
        self.total_frames.load(Ordering::Relaxed)
    }

    /// Number of currently free frames.
    pub fn free_frames(&self) -> u64 {
        self.free_frames.load(Ordering::Relaxed)
    }

    /// Number of currently allocated frames.
    pub fn used_frames(&self) -> u64 {
        self.total_frames() - self.free_frames()
    }

    /// Free frame count for a specific zone.
    pub fn zone_free_frames(&self, zone: Zone) -> u64 {
        self.zones[zone.index()].lock().count
    }

    /// Physical-to-virtual translation via HHDM.
    pub fn phys_to_virt(&self, phys: PhysAddr) -> *mut u8 {
        (phys.as_u64() + self.hhdm_offset) as *mut u8
    }

    /// HHDM offset.
    pub fn hhdm_offset(&self) -> u64 {
        self.hhdm_offset
    }

    fn push_frame(&self, zone: Zone, frame: PhysFrame) {
        let virt = (frame.start_address().as_u64() + self.hhdm_offset) as *mut FreeNode;
        let mut list = self.zones[zone.index()].lock();

        // SAFETY: Frame is free and mapped via HHDM. We store the free-list
        // node in the frame's own memory.
        unsafe {
            (*virt).next = list.head;
        }
        list.head = virt;
        list.count += 1;
    }

    fn pop_frame(&self, zone: Zone) -> Option<PhysFrame> {
        let mut list = self.zones[zone.index()].lock();
        if list.head.is_null() {
            return None;
        }

        let node = list.head;
        // SAFETY: Node is in a free frame mapped via HHDM.
        unsafe {
            list.head = (*node).next;
        }
        list.count -= 1;

        // Convert virtual address back to physical
        let virt_addr = node as u64;
        let phys_addr = virt_addr - self.hhdm_offset;
        Some(PhysFrame::containing_address(PhysAddr::new(phys_addr)))
    }
}

/// Simplified memory region for PMM initialization.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub kind: MemoryRegionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    KernelAndModules,
    Framebuffer,
}

impl MemoryRegionKind {
    pub fn from_limine(entry_type: limine::memory_map::EntryType) -> Self {
        match entry_type {
            limine::memory_map::EntryType::USABLE => MemoryRegionKind::Usable,
            limine::memory_map::EntryType::RESERVED => MemoryRegionKind::Reserved,
            limine::memory_map::EntryType::ACPI_RECLAIMABLE => MemoryRegionKind::AcpiReclaimable,
            limine::memory_map::EntryType::ACPI_NVS => MemoryRegionKind::AcpiNvs,
            limine::memory_map::EntryType::BAD_MEMORY => MemoryRegionKind::BadMemory,
            limine::memory_map::EntryType::BOOTLOADER_RECLAIMABLE => {
                MemoryRegionKind::BootloaderReclaimable
            }
            limine::memory_map::EntryType::EXECUTABLE_AND_MODULES => {
                MemoryRegionKind::KernelAndModules
            }
            limine::memory_map::EntryType::FRAMEBUFFER => MemoryRegionKind::Framebuffer,
            _ => MemoryRegionKind::Reserved,
        }
    }
}
