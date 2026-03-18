#![no_std]
#![no_main]

#[macro_use]
mod drivers;
mod arch;
mod memory;
mod panic;
pub mod test_framework;

use limine::request::{
    BootloaderInfoRequest, ExecutableAddressRequest, FramebufferRequest, HhdmRequest,
    MemoryMapRequest, MpRequest, RequestsEndMarker, RequestsStartMarker, RsdpRequest,
    StackSizeRequest,
};
use limine::BaseRevision;

// Limine protocol: BaseRevision must be present for the bootloader to
// recognize our kernel. It lives outside the requests section.
#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Request section markers and all requests must be in a contiguous section.
// The start marker must come first and end marker last.
#[used]
#[link_section = ".requests_start_marker"]
static _REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".requests"]
static _STACK_SIZE: StackSizeRequest = StackSizeRequest::new().with_size(0x10_0000);

#[used]
#[link_section = ".requests"]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[link_section = ".requests"]
static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
static KERNEL_ADDRESS: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static RSDP: RsdpRequest = RsdpRequest::new();

#[used]
#[link_section = ".requests"]
static SMP: MpRequest = MpRequest::new();

#[used]
#[link_section = ".requests_end_marker"]
static _REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

fn kernel_main() -> ! {
    // Serial port is the first thing we initialize — all debug output depends on it
    drivers::serial::init();

    serial_println!("LogOS v0.1.0 booting...");

    // Verify the bootloader supports our required revision
    if !BASE_REVISION.is_supported() {
        serial_println!("FATAL: Limine base revision not supported");
        halt_loop();
    }

    serial_println!("Bootloader revision: OK");

    // Validate CPU features
    let features = arch::x86_64::cpu::CpuFeatures::detect();
    if let Some(missing) = features.validate() {
        panic!("CPU lacks required feature: {}", missing);
    }

    serial_println!(
        "CPU: vendor={} family={} model={} stepping={}",
        core::str::from_utf8(&features.vendor).unwrap_or("unknown"),
        features.family,
        features.model,
        features.stepping,
    );
    serial_println!("CPU features: OK");
    serial_println!(
        "  XSAVE: {} (area size: {} bytes)",
        features.has_xsave,
        features.xsave_area_size
    );
    serial_println!("  RDRAND: {}", features.has_rdrand);
    serial_println!("  1 GiB pages: {}", features.has_1gib_pages);

    // Initialize memory subsystem
    let hhdm_offset = HHDM
        .get_response()
        .expect("HHDM not provided by bootloader")
        .offset();
    serial_println!("HHDM offset: {:#x}", hhdm_offset);

    let mmap_response = MEMORY_MAP
        .get_response()
        .expect("Memory map not provided by bootloader");
    let entries = mmap_response.entries();

    // Convert Limine memory map to our format
    let mut regions = [memory::pmm::MemoryRegion {
        base: 0,
        length: 0,
        kind: memory::pmm::MemoryRegionKind::Reserved,
    }; 128];
    let mut region_count = 0;
    let mut total_usable: u64 = 0;

    for entry in entries {
        if region_count >= regions.len() {
            break;
        }
        regions[region_count] = memory::pmm::MemoryRegion {
            base: entry.base,
            length: entry.length,
            kind: memory::pmm::MemoryRegionKind::from_limine(entry.entry_type),
        };
        if entry.entry_type == limine::memory_map::EntryType::USABLE {
            total_usable += entry.length;
        }
        region_count += 1;
    }

    serial_println!(
        "Memory: {} MiB usable ({} entries in map)",
        total_usable / (1024 * 1024),
        entries.len(),
    );

    // Initialize PMM
    // SAFETY: Called once during single-threaded boot. HHDM offset is from
    // the bootloader and memory map entries are accurate.
    unsafe {
        memory::pmm::Pmm::init(hhdm_offset, &regions[..region_count]);
    }

    let pmm = memory::pmm::Pmm::get();
    serial_println!(
        "PMM: {} frames ({} MiB) managed, {} free",
        pmm.total_frames(),
        pmm.total_frames() * 4096 / (1024 * 1024),
        pmm.free_frames(),
    );
    serial_println!(
        "  DMA16: {} frames, DMA32: {} frames, Normal: {} frames",
        pmm.zone_free_frames(memory::pmm::Zone::Dma16),
        pmm.zone_free_frames(memory::pmm::Zone::Dma32),
        pmm.zone_free_frames(memory::pmm::Zone::Normal),
    );

    // Initialize VMM using the bootloader's page tables
    // SAFETY: Called once during single-threaded boot, after PMM.
    unsafe {
        memory::vmm::Vmm::init(hhdm_offset);
    }
    serial_println!("VMM: initialized");

    // Initialize the kernel heap
    // SAFETY: Called once during single-threaded boot, after PMM and VMM.
    unsafe {
        memory::heap::init();
    }
    serial_println!(
        "Heap: initialized at {}",
        memory::vmm::layout::KERNEL_HEAP_START
    );

    // Activate slab allocator for small allocations
    memory::slab::activate();

    // Run memory integration tests
    memory_tests();

    if let Some(response) = KERNEL_ADDRESS.get_response() {
        serial_println!(
            "Kernel: phys={:#x} virt={:#x}",
            response.physical_base(),
            response.virtual_base()
        );
    }

    serial_println!("Boot complete. Halting.");

    halt_loop()
}

fn memory_tests() {
    extern crate alloc;
    use alloc::boxed::Box;
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use alloc::vec::Vec;

    // PMM: alloc and dealloc cycle
    {
        let pmm = memory::pmm::Pmm::get();
        let free_before = pmm.free_frames();

        let frame = pmm.alloc().expect("PMM alloc failed");
        assert_eq!(pmm.free_frames(), free_before - 1);

        // SAFETY: Frame was just allocated and is not in use.
        unsafe { pmm.dealloc(frame) };
        assert_eq!(pmm.free_frames(), free_before);

        serial_println!("TEST PMM alloc/dealloc: PASS");
    }

    // PMM: allocate many frames and verify they are unique and zeroed
    {
        let pmm = memory::pmm::Pmm::get();
        let count = 100;
        let mut frames = [memory::addr::PhysFrame::from_number(0); 100];
        let free_before = pmm.free_frames();

        for frame in frames.iter_mut().take(count) {
            *frame = pmm.alloc().expect("PMM exhausted during multi-alloc");
        }
        assert_eq!(pmm.free_frames(), free_before - count as u64);

        // Verify all frames are unique
        for i in 0..count {
            for j in (i + 1)..count {
                assert!(frames[i] != frames[j], "PMM returned duplicate frame");
            }
        }

        // Verify frames are zeroed (via HHDM)
        for frame in frames.iter().take(count) {
            let ptr = pmm.phys_to_virt(frame.start_address());
            for offset in 0..4096u64 {
                // SAFETY: Frame is allocated and mapped via HHDM.
                let byte = unsafe { *ptr.add(offset as usize) };
                assert_eq!(byte, 0, "PMM frame not zeroed");
            }
        }

        // Free all
        for frame in frames.iter().take(count) {
            // SAFETY: Frames were allocated above and are not in use.
            unsafe { pmm.dealloc(*frame) };
        }
        assert_eq!(pmm.free_frames(), free_before);

        serial_println!("TEST PMM multi-alloc (100 frames, unique, zeroed): PASS");
    }

    // Allocator: Box, Vec, String, BTreeMap
    {
        let b = Box::new(42u64);
        assert_eq!(*b, 42);
        drop(b);

        let mut v: Vec<u64> = Vec::new();
        for i in 0..1000 {
            v.push(i);
        }
        assert_eq!(v.len(), 1000);
        assert_eq!(v[999], 999);
        drop(v);

        let s = String::from("LogOS allocator works correctly");
        assert_eq!(s.len(), 31);
        drop(s);

        let mut map = BTreeMap::new();
        for i in 0..200 {
            map.insert(i, i * 3);
        }
        assert_eq!(map.get(&100), Some(&300));
        assert_eq!(map.len(), 200);
        drop(map);

        serial_println!("TEST alloc types (Box, Vec, String, BTreeMap): PASS");
    }

    // Stress test: 1000 allocations with pattern fill and verify
    {
        let pmm = memory::pmm::Pmm::get();
        let free_before = pmm.free_frames();

        let mut vecs: Vec<Vec<u8>> = Vec::new();
        for i in 0u8..200 {
            let size = 32 + (i as usize % 8) * 64;
            let mut v = Vec::with_capacity(size);
            for j in 0..size {
                v.push(i.wrapping_add(j as u8));
            }
            vecs.push(v);
        }

        // Verify patterns
        for (i, v) in vecs.iter().enumerate() {
            let i = i as u8;
            let size = 32 + (i as usize % 8) * 64;
            assert_eq!(v.len(), size);
            for (j, &byte) in v.iter().enumerate() {
                assert_eq!(byte, i.wrapping_add(j as u8), "Pattern mismatch");
            }
        }

        // Free all
        drop(vecs);

        // PMM should have recovered the frames (approximately — slab hysteresis may hold some)
        let free_after = pmm.free_frames();
        let leaked = free_before.saturating_sub(free_after);
        assert!(
            leaked <= 20,
            "Potential memory leak: {} frames not returned",
            leaked
        );

        serial_println!("TEST stress alloc (200 vecs, pattern fill, verify): PASS");
    }

    serial_println!("All memory tests passed.");
}

/// Enter an infinite halt loop.
pub fn halt_loop() -> ! {
    loop {
        arch::x86_64::cpu::hlt();
    }
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    panic::panic_handler(info)
}

#[no_mangle]
extern "C" fn _start() -> ! {
    kernel_main()
}
