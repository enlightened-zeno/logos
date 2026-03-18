#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
mod drivers;
#[allow(dead_code)]
mod arch;
#[allow(dead_code)]
mod entropy;
#[allow(dead_code)]
mod fs;
mod memory;
mod panic;
#[allow(dead_code)]
mod process;
pub mod sched;
#[allow(dead_code)]
mod shell;
#[allow(dead_code)]
mod sync;
#[allow(dead_code)]
mod syscall;
pub mod test_framework;
#[allow(dead_code)]
mod tty;

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

    // Initialize GDT and TSS
    // SAFETY: Called once during single-threaded boot.
    unsafe {
        arch::x86_64::gdt::init();
    }
    serial_println!("GDT: loaded");

    // Disable legacy PIC before setting up APIC
    // SAFETY: Called during boot before APIC init.
    unsafe {
        arch::x86_64::pic::disable();
    }

    // Initialize IDT
    // SAFETY: Called once after GDT is loaded.
    unsafe {
        arch::x86_64::idt::init();
    }
    serial_println!("IDT: loaded (exceptions 0-20)");

    // Initialize Local APIC
    // SAFETY: Called once after GDT/IDT/memory are ready.
    unsafe {
        arch::x86_64::apic::init(hhdm_offset);
    }

    // Calibrate and start the APIC timer
    // SAFETY: Called after APIC init, interrupts still disabled.
    unsafe {
        arch::x86_64::apic::calibrate_timer();
    }

    // Wire up the timer interrupt handler
    // SAFETY: Handler has correct x86-interrupt calling convention.
    unsafe {
        arch::x86_64::idt::set_handler(
            arch::x86_64::apic::TIMER_VECTOR,
            arch::x86_64::interrupts::timer_handler as *const () as u64,
            0,
        );
    }

    // Start periodic timer and enable interrupts
    arch::x86_64::apic::start_periodic();
    arch::x86_64::cpu::sti();
    serial_println!("APIC timer: running at ~1000 Hz");

    // Brief spin to verify tick counter advances
    let t0 = arch::x86_64::apic::ticks();
    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }
    let t1 = arch::x86_64::apic::ticks();
    serial_println!("Ticks after spin: {} (delta={})", t1, t1 - t0);
    assert!(t1 > t0, "APIC timer not ticking");
    serial_println!("TEST APIC timer ticking: PASS");

    // Initialize CSPRNG
    // SAFETY: Called once during boot.
    unsafe {
        entropy::init();
    }
    let r = entropy::random_u64();
    serial_println!("Entropy: random sample = {:#x}", r);
    assert!(r != 0, "CSPRNG returned zero");
    serial_println!("TEST entropy: PASS");

    // Initialize SYSCALL/SYSRET
    // SAFETY: Called once after GDT is loaded.
    unsafe {
        arch::x86_64::syscall::init();
    }

    // Initialize scheduler
    sched::init();

    // Spawn a test task
    use core::sync::atomic::{AtomicBool, Ordering};
    static TEST_TASK_RAN: AtomicBool = AtomicBool::new(false);

    fn test_task() -> ! {
        TEST_TASK_RAN.store(true, Ordering::Release);
        crate::serial_println!("TEST scheduler task switch: PASS");
        // Halt this task — scheduler will only run the boot task
        loop {
            crate::arch::x86_64::cpu::hlt();
        }
    }

    sched::spawn(test_task);

    // Yield to let the test task run
    sched::yield_now();

    // We should return here after the test task yields back
    assert!(
        TEST_TASK_RAN.load(Ordering::Acquire),
        "Test task did not run"
    );

    // Initialize VFS and mount filesystems
    fs::vfs::Vfs::init();
    fs::vfs::Vfs::mount("/", fs::tmpfs::TmpFs::new());
    fs::vfs::Vfs::mount("/dev", fs::devfs::DevFs::new());
    fs::vfs::Vfs::mount("/proc", fs::procfs::ProcFs::new());
    fs::vfs::Vfs::mount("/tmp", fs::tmpfs::TmpFs::new());

    // Initialize TTY subsystem
    tty::init();

    // Wire up keyboard interrupt (IRQ1 → vector 0x21)
    // SAFETY: Handler has correct x86-interrupt calling convention.
    unsafe {
        arch::x86_64::idt::set_handler(
            0x21,
            arch::x86_64::interrupts::keyboard_handler as *const () as u64,
            0,
        );
    }
    serial_println!("Keyboard: IRQ1 handler installed");

    // VFS tests
    vfs_tests();

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

    // Initialize shell CWD and launch the interactive shell
    shell::builtins::init_cwd();
    shell::run()
}

fn vfs_tests() {
    use fs::vfs::{InodeType, Vfs};

    // Test: resolve root
    let root = Vfs::resolve("/").expect("resolve / failed");
    assert_eq!(root.inode_type(), InodeType::Directory);
    serial_println!("TEST VFS resolve root: PASS");

    // Test: resolve /dev/null
    let null = Vfs::resolve("/dev/null").expect("resolve /dev/null failed");
    assert_eq!(null.inode_type(), InodeType::CharDevice);
    serial_println!("TEST VFS resolve /dev/null: PASS");

    // Test: write and read /dev/null
    let written = null.write(0, b"discarded").expect("write to null failed");
    assert_eq!(written, 9);
    let mut buf = [0u8; 16];
    let read = null.read(0, &mut buf).expect("read from null failed");
    assert_eq!(read, 0); // EOF
    serial_println!("TEST /dev/null read/write: PASS");

    // Test: /dev/zero
    let zero = Vfs::resolve("/dev/zero").expect("resolve /dev/zero failed");
    let mut buf = [0xFFu8; 32];
    let read = zero.read(0, &mut buf).expect("read from zero failed");
    assert_eq!(read, 32);
    assert!(buf.iter().all(|&b| b == 0));
    serial_println!("TEST /dev/zero: PASS");

    // Test: tmpfs create, write, read
    let root = Vfs::resolve("/tmp").expect("resolve /tmp failed");
    let file = root
        .create("hello.txt", InodeType::File, 0o644)
        .expect("create file failed");
    let written = file.write(0, b"Hello, LogOS!").expect("write failed");
    assert_eq!(written, 13);
    let mut buf = [0u8; 32];
    let read = file.read(0, &mut buf).expect("read failed");
    assert_eq!(read, 13);
    assert_eq!(&buf[..13], b"Hello, LogOS!");
    serial_println!("TEST tmpfs create/write/read: PASS");

    // Test: tmpfs readdir
    let entries = root.readdir().expect("readdir failed");
    assert!(entries.iter().any(|e| e.name == "hello.txt"));
    serial_println!("TEST tmpfs readdir: PASS");

    // Test: /proc/version
    let version = Vfs::resolve("/proc/version").expect("resolve /proc/version failed");
    let mut buf = [0u8; 64];
    let read = version.read(0, &mut buf).expect("read version failed");
    let content = core::str::from_utf8(&buf[..read]).expect("invalid utf8");
    assert!(content.contains("LogOS"));
    serial_println!("TEST /proc/version: PASS");

    // Test: /proc/meminfo
    let meminfo = Vfs::resolve("/proc/meminfo").expect("resolve /proc/meminfo failed");
    let mut buf = [0u8; 256];
    let read = meminfo.read(0, &mut buf).expect("read meminfo failed");
    let content = core::str::from_utf8(&buf[..read]).expect("invalid utf8");
    assert!(content.contains("MemTotal"));
    assert!(content.contains("MemFree"));
    serial_println!("TEST /proc/meminfo: PASS");

    serial_println!("All VFS tests passed.");
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
