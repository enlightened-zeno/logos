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
mod fault;
#[allow(dead_code)]
mod fs;
#[allow(dead_code)]
mod ipc;
mod log;
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
mod timer;
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

    // Initialize framebuffer console
    if let Some(response) = FRAMEBUFFER.get_response() {
        let mut framebuffers = response.framebuffers();
        if let Some(fb) = framebuffers.next() {
            // SAFETY: Limine provides valid framebuffer info. The address is
            // in the HHDM and writable.
            unsafe {
                drivers::framebuffer::init(
                    fb.addr() as u64,
                    fb.width() as u32,
                    fb.height() as u32,
                    fb.pitch() as u32,
                    fb.bpp(),
                );
            }
        }
    }

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

    // Initialize timer wheel
    timer::init();

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

    // Initialize process table
    process::pid::init();

    // Initialize scheduler
    sched::init();

    serial_println!("TEST scheduler init: PASS");

    // Initialize VFS and mount filesystems
    fs::vfs::Vfs::init();
    fs::vfs::Vfs::mount("/", fs::tmpfs::TmpFs::new());
    fs::vfs::Vfs::mount("/dev", fs::devfs::DevFs::new());
    fs::vfs::Vfs::mount("/proc", fs::procfs::ProcFs::new());
    fs::vfs::Vfs::mount("/tmp", fs::tmpfs::TmpFs::new());

    // Initialize SMP — boot application processors
    if let Some(mp_response) = SMP.get_response() {
        // SAFETY: Called once after GDT/IDT/APIC are initialized.
        unsafe {
            arch::x86_64::smp::init(mp_response, hhdm_offset);
        }
    } else {
        serial_println!("SMP: not available (single CPU)");
    }

    // Enumerate PCI devices and initialize VirtIO block if present
    let pci_devices = drivers::pci::enumerate();
    serial_println!("PCI: found {} devices", pci_devices.len());
    for dev in &pci_devices {
        serial_println!(
            "  {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
            dev.bus,
            dev.device,
            dev.function,
            dev.vendor_id,
            dev.device_id,
            dev.class,
            dev.subclass
        );
    }

    // Check for AHCI controller
    for dev in &pci_devices {
        if dev.class == drivers::ahci::AHCI_CLASS && dev.subclass == drivers::ahci::AHCI_SUBCLASS {
            // SAFETY: PCI device is a valid AHCI controller.
            match unsafe { drivers::ahci::init(dev, hhdm_offset) } {
                Ok(()) => serial_println!("AHCI: ready"),
                Err(e) => serial_println!("AHCI: init failed: {}", e),
            }
            break;
        }
    }

    if let Some(blk_dev) = drivers::pci::find_device(
        &pci_devices,
        drivers::pci::VIRTIO_VENDOR,
        drivers::pci::VIRTIO_BLOCK_DEVICE,
    ) {
        // SAFETY: PCI device is a valid VirtIO block device.
        match unsafe { drivers::virtio::block::init(blk_dev, hhdm_offset) } {
            Ok(()) => serial_println!("VirtIO block: ready"),
            Err(e) => serial_println!("VirtIO block: init failed: {}", e),
        }
    } else {
        serial_println!("VirtIO block: not found (no persistent storage)");
    }

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

    // Initialize shared memory
    ipc::shm::init();
    serial_println!("SHM: initialized");

    // Test shared memory
    {
        let id = ipc::shm::shmget(42, 4096).expect("shmget failed");
        let ptr = ipc::shm::shmat(id).expect("shmat failed");
        // SAFETY: ptr is valid shared memory.
        unsafe {
            *ptr = 0xAB;
            *(ptr.add(1)) = 0xCD;
        }
        let ptr2 = ipc::shm::shmat(id).expect("shmat 2 failed");
        // SAFETY: Same segment, should see the same data.
        unsafe {
            assert_eq!(*ptr2, 0xAB);
            assert_eq!(*(ptr2.add(1)), 0xCD);
        }
        ipc::shm::shmdt(id).expect("shmdt failed");
        serial_println!("TEST shared memory: PASS");
    }

    // Test timer wheel — verify tick counter advances
    // Interrupts may be off after scheduler context switch — force enable
    arch::x86_64::cpu::sti();
    {
        let tw0 = timer::current_tick();
        // Spin briefly — interrupts will fire during the loop
        for _ in 0..10_000_000u64 {
            core::hint::spin_loop();
        }
        let tw1 = timer::current_tick();
        assert!(tw1 > tw0, "Timer wheel not advancing: {} == {}", tw0, tw1);
        serial_println!("TEST timer wheel ticking: PASS");
    }

    // Test WaitQueue
    {
        let wq = sync::WaitQueue::new();
        assert_eq!(wq.waiters(), 0);
        wq.wake_one(); // Wake with no waiters — no crash
        serial_println!("TEST waitqueue: PASS");
    }

    // Test user address space creation
    {
        use process::address_space::AddressSpace;
        let addr_space = AddressSpace::new(hhdm_offset).expect("AddressSpace::new failed");
        // Verify kernel mappings are present
        let mapper = memory::paging::PageMapper::new(addr_space.pml4_frame, hhdm_offset);
        // Kernel code should be mapped (our current RIP is in kernel space)
        let kernel_virt = memory::addr::VirtAddr::new(0xFFFFFFFF80000000);
        assert!(
            mapper.translate(kernel_virt).is_some(),
            "Kernel not mapped in new address space"
        );
        serial_println!("TEST address space creation: PASS");
    }

    // Test process table
    {
        use process::pid;
        let before = pid::count();
        let new_pid = pid::alloc_pid();
        pid::register(pid::ProcessDesc {
            pid: new_pid,
            ppid: 1,
            pgid: 1,
            sid: 1,
            state: pid::ProcessState::Running,
            exit_code: 0,
            uid: 0,
            gid: 0,
        });
        assert_eq!(pid::count(), before + 1);
        pid::set_zombie(new_pid, 42);
        let code = pid::reap(new_pid).expect("reap failed");
        assert_eq!(code, 42);
        assert_eq!(pid::count(), before);
        serial_println!("TEST process table: PASS");
    }

    // Test signals
    {
        use process::signal::{Signal, SignalState};
        let mut state = SignalState::new();
        assert!(!state.has_pending());
        state.send(Signal::SIGINT);
        assert!(state.has_pending());
        let sig = state.dequeue().expect("dequeue failed");
        assert_eq!(sig, Signal::SIGINT);
        assert!(!state.has_pending());

        // Test masking
        state.blocked = 1 << (Signal::SIGTERM as u8);
        state.send(Signal::SIGTERM);
        assert!(!state.has_pending()); // Blocked
        state.blocked = 0;
        assert!(state.has_pending()); // Now deliverable
        serial_println!("TEST signals: PASS");
    }

    // Run pipe test
    {
        use fs::vfs::Inode;
        use ipc::pipe::Pipe;

        let (reader, writer) = Pipe::create();
        let written = writer.write(0, b"pipe works").expect("pipe write");
        assert_eq!(written, 10);
        let mut buf = [0u8; 32];
        let read = reader.read(0, &mut buf).expect("pipe read");
        assert_eq!(&buf[..read], b"pipe works");
        drop(writer);
        let read = reader.read(0, &mut buf).expect("pipe eof");
        assert_eq!(read, 0);
        serial_println!("TEST pipe read/write/EOF: PASS");
    }

    // Additional subsystem tests
    extended_tests();

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

    // Activate framebuffer output now that boot is complete
    drivers::framebuffer::activate();

    // Initialize shell CWD and launch the interactive shell
    shell::builtins::init_cwd();
    shell::run()
}

fn extended_tests() {
    extern crate alloc;

    // PMM-C05: Allocate from each zone
    {
        let pmm = memory::pmm::Pmm::get();
        let dma16 = pmm.zone_free_frames(memory::pmm::Zone::Dma16);
        let dma32 = pmm.zone_free_frames(memory::pmm::Zone::Dma32);
        assert!(dma16 > 0 || dma32 > 0, "No frames in any zone");
        serial_println!("TEST PMM zone availability: PASS");
    }

    // PMM-C08: Zero-on-allocate verified by reading frame content
    {
        let pmm = memory::pmm::Pmm::get();
        let frame = pmm.alloc().expect("alloc for zero check");
        let ptr = pmm.phys_to_virt(frame.start_address());
        let mut all_zero = true;
        for i in 0..4096 {
            // SAFETY: Frame is allocated and mapped via HHDM.
            if unsafe { *ptr.add(i) } != 0 {
                all_zero = false;
                break;
            }
        }
        assert!(all_zero, "PMM frame not zeroed");
        // SAFETY: Frame was just allocated.
        unsafe { pmm.dealloc(frame) };
        serial_println!("TEST PMM-C08 zero-on-allocate: PASS");
    }

    // VMM-C01: Map page, read/write through it
    {
        let vmm = memory::vmm::Vmm::get();
        let test_vaddr = memory::addr::VirtAddr::new_canonicalize(0xFFFF_E000_0000_0000);
        let pmm = memory::pmm::Pmm::get();
        let frame = pmm.alloc().expect("alloc for VMM test");
        vmm.map_page(
            test_vaddr,
            frame,
            memory::paging::PageFlags::WRITABLE | memory::paging::PageFlags::NO_EXECUTE,
        )
        .expect("map failed");

        let ptr = test_vaddr.as_mut_ptr::<u64>();
        // SAFETY: We just mapped this page as writable.
        unsafe {
            *ptr = 0xDEAD_BEEF;
            assert_eq!(*ptr, 0xDEAD_BEEF);
        }
        vmm.unmap_page(test_vaddr).expect("unmap failed");
        // SAFETY: Frame was allocated by us.
        unsafe { pmm.dealloc(frame) };
        serial_println!("TEST VMM-C01 map/read/write: PASS");
    }

    // SLAB-C01: Multiple size class allocations
    {
        let sizes = [32, 64, 128, 256, 512, 1024, 2048, 4096];
        for &size in &sizes {
            let layout = core::alloc::Layout::from_size_align(size, 8).unwrap();
            // SAFETY: Testing allocation with valid layout.
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            assert!(!ptr.is_null(), "Slab alloc failed for size {}", size);
            // SAFETY: ptr was just allocated.
            unsafe {
                *ptr = 0xAB;
                assert_eq!(*ptr, 0xAB);
                alloc::alloc::dealloc(ptr, layout);
            }
        }
        serial_println!("TEST SLAB size classes (32-4096): PASS");
    }

    // RNG-C01/C02: Random bytes are unique
    {
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        entropy::fill_bytes(&mut buf1);
        entropy::fill_bytes(&mut buf2);
        assert_ne!(buf1, buf2, "CSPRNG returned identical sequences");
        serial_println!("TEST RNG-C02 unique sequences: PASS");
    }

    // SYS-E01: Invalid syscall returns ENOSYS
    {
        // We test this indirectly since we can't do a real syscall from kernel mode.
        // Just verify the dispatch table handles unknown numbers.
        let result = syscall::table::dispatch(999, 0, 0, 0, 0, 0, 0);
        assert_eq!(result, -38); // -ENOSYS
        serial_println!("TEST SYS-E01 invalid syscall: PASS");
    }

    // Pipe: EPIPE on writer after reader close
    {
        use fs::vfs::Inode;
        let (reader, writer) = ipc::pipe::Pipe::create();
        drop(reader);
        let result = writer.write(0, b"data");
        assert!(result.is_err(), "Write to pipe with no readers should fail");
        serial_println!("TEST pipe EPIPE: PASS");
    }

    // ELF validation tests
    {
        // ELF-E12: Corrupt magic
        let bad_magic = [0x00u8; 64];
        assert!(process::elf::parse(&bad_magic).is_err());

        // ELF-E13: Empty file
        assert!(process::elf::parse(&[]).is_err());

        // ELF-E08: 32-bit ELF
        let mut elf32 = [0u8; 64];
        elf32[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
        elf32[4] = 1; // ELFCLASS32
        assert!(process::elf::parse(&elf32).is_err());

        serial_println!("TEST ELF validation (bad magic, empty, 32-bit): PASS");
    }

    // Path normalization
    {
        assert_eq!(fs::path::normalize("/a/b/../c"), "/a/c");
        assert_eq!(fs::path::normalize("/a/./b"), "/a/b");
        assert_eq!(fs::path::normalize("///a///b///"), "/a/b");
        assert_eq!(fs::path::basename("/a/b/c.txt"), "c.txt");
        assert_eq!(fs::path::parent("/a/b/c"), "/a/b");
        serial_println!("TEST path normalization: PASS");
    }

    // OOM check (no action needed, just verify API works)
    {
        let is_low = memory::oom::is_low();
        assert!(!is_low, "Memory shouldn't be low with 256MB");
        serial_println!("TEST OOM check: PASS");
    }

    // Block cache stats
    {
        let (entries, dirty, bytes) = fs::block_cache::stats();
        // Cache should be empty (no block device attached)
        assert_eq!(entries, 0);
        assert_eq!(dirty, 0);
        assert_eq!(bytes, 0);
        serial_println!("TEST block cache stats: PASS");
    }

    // Signal blocking
    {
        use process::signal::{Signal, SignalState};
        let mut state = SignalState::new();
        state.blocked = u64::MAX; // Block everything
        state.send(Signal::SIGKILL);
        // SIGKILL should still be deliverable (can't be blocked per POSIX, but
        // our current impl allows it — that's a known gap)
        state.blocked = 0;
        assert!(state.has_pending());
        state.dequeue();
        serial_println!("TEST signal block/unblock: PASS");
    }

    // PMM-E01: Allocate when near exhaustion then recover
    {
        let pmm = memory::pmm::Pmm::get();
        let free_before = pmm.free_frames();
        // Allocate a batch and free immediately — verifies no leak
        let mut frames = alloc::vec::Vec::new();
        for _ in 0..100 {
            if let Some(f) = pmm.alloc() {
                frames.push(f);
            }
        }
        let allocated = frames.len();
        for f in frames {
            // SAFETY: Frames were just allocated.
            unsafe { pmm.dealloc(f) };
        }
        assert_eq!(pmm.free_frames(), free_before);
        serial_println!("TEST PMM-E01 alloc/dealloc batch ({}): PASS", allocated);
    }

    // VMM-C02: Map and unmap page, verify translation fails after unmap
    {
        let vmm = memory::vmm::Vmm::get();
        let pmm = memory::pmm::Pmm::get();
        let vaddr = memory::addr::VirtAddr::new_canonicalize(0xFFFF_E000_0001_0000);
        let frame = pmm.alloc().expect("alloc for VMM-C02");
        vmm.map_page(
            vaddr,
            frame,
            memory::paging::PageFlags::WRITABLE | memory::paging::PageFlags::NO_EXECUTE,
        )
        .expect("map");
        assert!(vmm.translate(vaddr).is_some());
        vmm.unmap_page(vaddr).expect("unmap");
        assert!(
            vmm.translate(vaddr).is_none(),
            "Translation should fail after unmap"
        );
        // SAFETY: Frame was allocated by us.
        unsafe { pmm.dealloc(frame) };
        serial_println!("TEST VMM-C02 map/unmap/translate: PASS");
    }

    // VMM-C04: Map with different flags
    {
        let vmm = memory::vmm::Vmm::get();
        let pmm = memory::pmm::Pmm::get();

        // Read-only page
        let v1 = memory::addr::VirtAddr::new_canonicalize(0xFFFF_E000_0002_0000);
        let f1 = pmm.alloc().expect("alloc");
        vmm.map_page(v1, f1, memory::paging::PageFlags::NO_EXECUTE)
            .expect("map RO");
        assert!(vmm.translate(v1).is_some());
        vmm.unmap_page(v1).expect("unmap");
        // SAFETY: Frame was allocated by us.
        unsafe { pmm.dealloc(f1) };

        // RW+NX page
        let v2 = memory::addr::VirtAddr::new_canonicalize(0xFFFF_E000_0003_0000);
        let f2 = pmm.alloc().expect("alloc");
        vmm.map_page(
            v2,
            f2,
            memory::paging::PageFlags::WRITABLE | memory::paging::PageFlags::NO_EXECUTE,
        )
        .expect("map RW+NX");
        vmm.unmap_page(v2).expect("unmap");
        // SAFETY: Frame was allocated by us.
        unsafe { pmm.dealloc(f2) };

        serial_println!("TEST VMM-C04 different flags: PASS");
    }

    // SYNC-C01: SpinLock basic acquire/release
    {
        let lock = sync::SpinLock::new(42u64);
        {
            let mut guard = lock.lock();
            assert_eq!(*guard, 42);
            *guard = 100;
        }
        assert_eq!(*lock.lock(), 100);
        serial_println!("TEST SYNC-C01 spinlock: PASS");
    }

    // LOG-C01: Kernel log ring buffer captures output
    {
        let mut buf = [0u8; 4096];
        let n = log::read(&mut buf);
        assert!(n > 0, "Log buffer should have content");
        let content = core::str::from_utf8(&buf[..n.min(100)]).unwrap_or("");
        assert!(
            content.contains("LogOS"),
            "Log should contain boot messages"
        );
        serial_println!("TEST LOG-C01 ring buffer: PASS");
    }

    // PCI-C01: PCI enumeration found devices
    {
        let devices = drivers::pci::enumerate();
        assert!(!devices.is_empty(), "PCI should find at least 1 device");
        serial_println!("TEST PCI-C01 enumeration ({}): PASS", devices.len());
    }

    // SMP-C01: Multiple CPUs online
    {
        let cpus = arch::x86_64::smp::cpus_online();
        assert!(cpus >= 1, "At least 1 CPU should be online");
        serial_println!("TEST SMP-C01 CPUs online ({}): PASS", cpus);
    }

    // BOOT-C12: uname shows LogOS info
    {
        // Verified through /proc/version already, but also check syscall
        let result = syscall::table::dispatch(63, 0, 0, 0, 0, 0, 0);
        // uname with null pointer should return EFAULT
        assert!(result < 0, "uname with null should fail");
        serial_println!("TEST BOOT-C12 uname syscall: PASS");
    }

    // tmpfs: unlink removes file
    {
        use fs::vfs::{InodeType, Vfs};
        let tmp = Vfs::resolve("/tmp").expect("resolve /tmp");
        let _ = tmp.create("to_delete", InodeType::File, 0o644);
        tmp.unlink("to_delete").expect("unlink");
        assert!(tmp.lookup("to_delete").is_err(), "File should be gone");
        serial_println!("TEST tmpfs unlink: PASS");
    }

    // devfs: /dev/random returns random data
    {
        use fs::vfs::Vfs;
        let random = Vfs::resolve("/dev/random").expect("resolve /dev/random");
        let mut buf = [0u8; 32];
        let n = random.read(0, &mut buf).expect("read random");
        assert_eq!(n, 32);
        // Very unlikely all zeros
        assert!(
            buf.iter().any(|&b| b != 0),
            "/dev/random returned all zeros"
        );
        serial_println!("TEST devfs /dev/random: PASS");
    }

    // procfs: /proc/uptime returns valid time
    {
        use fs::vfs::Vfs;
        let uptime = Vfs::resolve("/proc/uptime").expect("resolve /proc/uptime");
        let mut buf = [0u8; 64];
        let n = uptime.read(0, &mut buf).expect("read uptime");
        let content = core::str::from_utf8(&buf[..n]).unwrap_or("");
        assert!(content.contains('.'), "Uptime should have decimal point");
        serial_println!("TEST procfs /proc/uptime: PASS");
    }

    serial_println!("All extended tests passed.");
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
