extern crate alloc;

use crate::fs::vfs::{InodeType, Vfs};
use crate::sync::SpinLock;
use alloc::string::String;

static CWD: SpinLock<String> = SpinLock::new(String::new());

pub fn init_cwd() {
    *CWD.lock() = String::from("/");
}

fn cwd() -> String {
    CWD.lock().clone()
}

/// Resolve a path relative to CWD.
fn resolve_path(path: &str) -> String {
    if path.starts_with('/') {
        crate::fs::path::normalize(path)
    } else {
        let cwd = cwd();
        let full = if cwd == "/" {
            alloc::format!("/{}", path)
        } else {
            alloc::format!("{}/{}", cwd, path)
        };
        crate::fs::path::normalize(&full)
    }
}

pub fn help() {
    crate::serial_println!("Available commands:");
    crate::serial_println!("  help              Show this help");
    crate::serial_println!("  echo [args...]    Print arguments");
    crate::serial_println!("  ls [path]         List directory contents");
    crate::serial_println!("  cat <file>        Print file contents");
    crate::serial_println!("  mkdir <dir>       Create directory");
    crate::serial_println!("  touch <file>      Create empty file");
    crate::serial_println!("  rm <file>         Remove file");
    crate::serial_println!("  write <file> <text> Write text to file");
    crate::serial_println!("  pwd               Print working directory");
    crate::serial_println!("  cd <dir>          Change directory");
    crate::serial_println!("  stat <path>       Show file information");
    crate::serial_println!("  free              Show memory usage");
    crate::serial_println!("  uptime            Show system uptime");
    crate::serial_println!("  uname             Show system information");
    crate::serial_println!("  ps                Show running tasks");
    crate::serial_println!("  hexdump <file>    Hex dump of file");
    crate::serial_println!("  dmesg             Show kernel log");
    crate::serial_println!("  clear             Clear screen");
    crate::serial_println!("  pipe-test         Test pipe IPC");
    crate::serial_println!("  stress [test]     Run stress tests (alloc, vfs, pipe, all)");
    crate::serial_println!("  bench [test]      Run benchmarks (pmm, slab, ctx, syscall, pipe)");
    crate::serial_println!("  leakcheck         Check for resource leaks");
    crate::serial_println!("  shutdown          Power off");
    crate::serial_println!("  reboot            Restart");
}

pub fn echo(args: &[String]) {
    let output: String = args.join(" ");
    crate::serial_println!("{}", output);
}

pub fn ls(args: &[String]) {
    let path = if args.is_empty() {
        cwd()
    } else {
        resolve_path(&args[0])
    };

    match Vfs::resolve(&path) {
        Ok(inode) => match inode.readdir() {
            Ok(entries) => {
                for entry in &entries {
                    let type_char = match entry.inode_type {
                        InodeType::Directory => 'd',
                        InodeType::File => '-',
                        InodeType::CharDevice => 'c',
                        InodeType::BlockDevice => 'b',
                        InodeType::Symlink => 'l',
                        InodeType::Pipe => 'p',
                    };
                    crate::serial_println!("{}  {}", type_char, entry.name);
                }
            }
            Err(_) => {
                // Not a directory, show the file itself
                if let Ok(st) = inode.stat() {
                    crate::serial_println!("{}", path);
                    crate::serial_println!("  size: {} bytes", st.size);
                }
            }
        },
        Err(e) => crate::serial_println!("ls: {}: {:?}", path, e),
    }
}

pub fn cat(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("cat: missing file argument");
        return;
    }

    let path = resolve_path(&args[0]);
    match Vfs::resolve(&path) {
        Ok(inode) => {
            let mut buf = [0u8; 4096];
            let mut offset = 0u64;
            loop {
                match inode.read(offset, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &byte in &buf[..n] {
                            crate::drivers::serial::write_byte(byte);
                        }
                        offset += n as u64;
                    }
                    Err(e) => {
                        crate::serial_println!("\ncat: read error: {:?}", e);
                        break;
                    }
                }
            }
            // Ensure we end on a newline
            if offset > 0 {
                crate::serial_println!();
            }
        }
        Err(e) => crate::serial_println!("cat: {}: {:?}", path, e),
    }
}

pub fn mkdir(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("mkdir: missing directory argument");
        return;
    }

    let path = resolve_path(&args[0]);
    let parent = crate::fs::path::parent(&path);
    let name = crate::fs::path::basename(&path);

    match Vfs::resolve(parent) {
        Ok(parent_inode) => match parent_inode.create(name, InodeType::Directory, 0o755) {
            Ok(_) => {}
            Err(e) => crate::serial_println!("mkdir: {}: {:?}", path, e),
        },
        Err(e) => crate::serial_println!("mkdir: {}: {:?}", parent, e),
    }
}

pub fn touch(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("touch: missing file argument");
        return;
    }

    let path = resolve_path(&args[0]);
    let parent = crate::fs::path::parent(&path);
    let name = crate::fs::path::basename(&path);

    match Vfs::resolve(parent) {
        Ok(parent_inode) => {
            // Create if doesn't exist, ignore if it does
            let _ = parent_inode.create(name, InodeType::File, 0o644);
        }
        Err(e) => crate::serial_println!("touch: {}: {:?}", parent, e),
    }
}

pub fn rm(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("rm: missing file argument");
        return;
    }

    let path = resolve_path(&args[0]);
    let parent = crate::fs::path::parent(&path);
    let name = crate::fs::path::basename(&path);

    match Vfs::resolve(parent) {
        Ok(parent_inode) => match parent_inode.unlink(name) {
            Ok(_) => {}
            Err(e) => crate::serial_println!("rm: {}: {:?}", path, e),
        },
        Err(e) => crate::serial_println!("rm: {}: {:?}", parent, e),
    }
}

pub fn write_file(args: &[String]) {
    if args.len() < 2 {
        crate::serial_println!("write: usage: write <file> <text>");
        return;
    }

    let path = resolve_path(&args[0]);
    let text: String = args[1..].join(" ");

    match Vfs::resolve(&path) {
        Ok(inode) => match inode.write(0, text.as_bytes()) {
            Ok(_) => {}
            Err(e) => crate::serial_println!("write: {}: {:?}", path, e),
        },
        Err(_) => {
            // Try to create the file first
            let parent = crate::fs::path::parent(&path);
            let name = crate::fs::path::basename(&path);
            match Vfs::resolve(parent) {
                Ok(parent_inode) => match parent_inode.create(name, InodeType::File, 0o644) {
                    Ok(inode) => match inode.write(0, text.as_bytes()) {
                        Ok(_) => {}
                        Err(e) => crate::serial_println!("write: {}: {:?}", path, e),
                    },
                    Err(e) => crate::serial_println!("write: {}: {:?}", path, e),
                },
                Err(e) => crate::serial_println!("write: {}: {:?}", parent, e),
            }
        }
    }
}

pub fn pwd() {
    crate::serial_println!("{}", cwd());
}

pub fn cd(args: &[String]) {
    let path = if args.is_empty() {
        String::from("/")
    } else {
        resolve_path(&args[0])
    };

    match Vfs::resolve(&path) {
        Ok(inode) => {
            if inode.inode_type() != InodeType::Directory {
                crate::serial_println!("cd: {}: Not a directory", path);
                return;
            }
            *CWD.lock() = path;
        }
        Err(e) => crate::serial_println!("cd: {}: {:?}", path, e),
    }
}

pub fn stat(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("stat: missing path argument");
        return;
    }

    let path = resolve_path(&args[0]);
    match Vfs::resolve(&path) {
        Ok(inode) => match inode.stat() {
            Ok(st) => {
                crate::serial_println!("  File: {}", path);
                crate::serial_println!("  Size: {} bytes", st.size);
                crate::serial_println!("  Type: {:?}", st.inode_type);
                crate::serial_println!(" Inode: {}", st.inode);
                crate::serial_println!(" Links: {}", st.nlink);
                crate::serial_println!("  Mode: {:o}", st.mode);
            }
            Err(e) => crate::serial_println!("stat: {}: {:?}", path, e),
        },
        Err(e) => crate::serial_println!("stat: {}: {:?}", path, e),
    }
}

pub fn free() {
    let pmm = crate::memory::pmm::Pmm::get();
    let total = pmm.total_frames() * 4;
    let free = pmm.free_frames() * 4;
    let used = total - free;

    crate::serial_println!("              total       used       free");
    crate::serial_println!(
        "Mem:    {:>10} {:>10} {:>10}",
        format_kb(total),
        format_kb(used),
        format_kb(free)
    );
}

fn format_kb(kb: u64) -> alloc::string::String {
    if kb >= 1024 {
        alloc::format!("{} MiB", kb / 1024)
    } else {
        alloc::format!("{} KiB", kb)
    }
}

pub fn uptime() {
    let ticks = crate::arch::x86_64::apic::ticks();
    let secs = ticks / 1000;
    let mins = secs / 60;
    let hours = mins / 60;

    if hours > 0 {
        crate::serial_println!("up {}h {}m {}s", hours, mins % 60, secs % 60);
    } else if mins > 0 {
        crate::serial_println!("up {}m {}s", mins, secs % 60);
    } else {
        crate::serial_println!("up {}s", secs);
    }
}

pub fn uname() {
    crate::serial_println!("LogOS 0.1.0 x86_64");
}

pub fn ps() {
    let cpus = crate::arch::x86_64::smp::cpus_online();
    let procs = crate::process::pid::list();
    crate::serial_println!("  PID  PPID  STATE");
    for (pid, ppid, state) in &procs {
        let state_str = match state {
            crate::process::pid::ProcessState::Running => "running",
            crate::process::pid::ProcessState::Sleeping => "sleeping",
            crate::process::pid::ProcessState::Zombie => "zombie",
            crate::process::pid::ProcessState::Stopped => "stopped",
        };
        crate::serial_println!("{:>5}  {:>4}  {}", pid, ppid, state_str);
    }
    crate::serial_println!();
    crate::serial_println!("{} processes, {} CPUs online", procs.len(), cpus);
}

pub fn clear() {
    // ANSI escape: clear screen and move cursor to top-left
    crate::serial_print!("\x1b[2J\x1b[H");
}

pub fn hexdump(args: &[String]) {
    if args.is_empty() {
        crate::serial_println!("hexdump: missing file argument");
        return;
    }

    let path = resolve_path(&args[0]);
    match Vfs::resolve(&path) {
        Ok(inode) => {
            let mut buf = [0u8; 256];
            match inode.read(0, &mut buf) {
                Ok(n) => {
                    for (i, chunk) in buf[..n].chunks(16).enumerate() {
                        crate::serial_print!("{:08x}  ", i * 16);
                        for (j, &byte) in chunk.iter().enumerate() {
                            crate::serial_print!("{:02x} ", byte);
                            if j == 7 {
                                crate::serial_print!(" ");
                            }
                        }
                        // Pad if less than 16 bytes
                        for j in chunk.len()..16 {
                            crate::serial_print!("   ");
                            if j == 7 {
                                crate::serial_print!(" ");
                            }
                        }
                        crate::serial_print!(" |");
                        for &byte in chunk {
                            if byte.is_ascii_graphic() || byte == b' ' {
                                crate::serial_print!("{}", byte as char);
                            } else {
                                crate::serial_print!(".");
                            }
                        }
                        crate::serial_println!("|");
                    }
                }
                Err(e) => crate::serial_println!("hexdump: {}: {:?}", path, e),
            }
        }
        Err(e) => crate::serial_println!("hexdump: {}: {:?}", path, e),
    }
}

pub fn dmesg() {
    let mut buf = [0u8; 4096];
    let n = crate::log::read(&mut buf);
    for &byte in &buf[..n] {
        crate::drivers::serial::write_byte(byte);
    }
}

pub fn shutdown() {
    crate::serial_println!("Shutting down...");
    // ACPI shutdown via QEMU debug exit
    crate::arch::x86_64::io::outw(0x604, 0x2000);
    // Fallback: halt
    loop {
        crate::arch::x86_64::cpu::hlt();
    }
}

pub fn reboot() {
    crate::serial_println!("Rebooting...");
    // Pulse the keyboard controller reset line
    crate::arch::x86_64::io::outb(0x64, 0xFE);
    // Fallback: halt
    loop {
        crate::arch::x86_64::cpu::hlt();
    }
}

pub fn run_user_program() {
    extern crate alloc;
    use crate::memory::paging::PageFlags;
    use crate::process::address_space::AddressSpace;

    static USER_ELF: &[u8] = include_bytes!("../test_user_program.bin");

    let info = crate::process::elf::parse(USER_ELF).expect("ELF parse failed");
    let hhdm = unsafe { crate::memory::vmm::layout::PHYS_MEM_OFFSET };
    let addr_space = AddressSpace::new(hhdm).expect("address space");

    for seg in &info.segments {
        let mut flags = PageFlags::USER;
        if seg.is_writable() {
            flags |= PageFlags::WRITABLE;
        }
        if !seg.is_executable() {
            flags |= PageFlags::NO_EXECUTE;
        }
        addr_space
            .load_segment(
                seg.vaddr,
                &USER_ELF[seg.offset as usize..(seg.offset + seg.filesz) as usize],
                seg.memsz,
                flags,
            )
            .expect("load segment");
    }

    addr_space.map_user_stack().expect("map user stack");

    // Set up kernel stack for syscall return
    let kernel_stack = alloc::vec![0u8; 32768];
    let kernel_stack_top = kernel_stack.as_ptr() as u64 + 32768;
    // SAFETY: Setting per-CPU kernel stack.
    unsafe {
        crate::arch::x86_64::percpu::init(kernel_stack_top);
    }

    crate::serial_println!("Entering user mode at {:#x}...", info.entry_point);

    // SAFETY: Address space is valid, user code loaded, stacks mapped.
    unsafe {
        core::arch::asm!(
            "mov cr3, {cr3}",
            "push {ss}",
            "push {rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "iretq",
            cr3 = in(reg) addr_space.cr3(),
            ss = in(reg) crate::arch::x86_64::gdt::USER_DS as u64,
            rsp = in(reg) addr_space.stack_top,
            rflags = in(reg) 0x202u64,
            cs = in(reg) crate::arch::x86_64::gdt::USER_CS as u64,
            rip = in(reg) info.entry_point,
            options(noreturn)
        );
    }
}

pub fn pipe_test() {
    use crate::fs::vfs::Inode;
    use crate::ipc::pipe::Pipe;

    let (reader, writer) = Pipe::create();

    // Write some data
    let data = b"Hello through the pipe!";
    let written = writer.write(0, data).expect("pipe write failed");
    assert_eq!(written, data.len());

    // Read it back
    let mut buf = [0u8; 64];
    let read = reader.read(0, &mut buf).expect("pipe read failed");
    assert_eq!(read, data.len());
    assert_eq!(&buf[..read], data);

    // Drop writer → reader should get EOF
    drop(writer);
    let read = reader
        .read(0, &mut buf)
        .expect("pipe read after close failed");
    assert_eq!(read, 0);

    crate::serial_println!("Pipe test: PASS");
}

pub fn stress_test(args: &[String]) {
    let test = if args.is_empty() { "all" } else { &args[0] };

    match test {
        "alloc" => stress_alloc(),
        "vfs" => stress_vfs(),
        "pipe" => stress_pipe(),
        "all" => {
            stress_alloc();
            stress_vfs();
            stress_pipe();
        }
        _ => crate::serial_println!(
            "stress: unknown test '{}'. Try: alloc, vfs, pipe, all",
            test
        ),
    }
}

fn stress_alloc() {
    extern crate alloc;
    use alloc::vec::Vec;

    crate::serial_println!("Stress: allocating 1000 vectors...");
    let pmm = crate::memory::pmm::Pmm::get();
    let free_before = pmm.free_frames();

    let mut vecs: Vec<Vec<u8>> = Vec::new();
    for i in 0u16..1000 {
        let size = 64 + (i as usize % 16) * 128;
        let mut v = Vec::with_capacity(size);
        for j in 0..size {
            v.push((i as u8).wrapping_add(j as u8));
        }
        vecs.push(v);
    }

    // Verify patterns
    for (i, v) in vecs.iter().enumerate() {
        let i = i as u16;
        let size = 64 + (i as usize % 16) * 128;
        assert_eq!(v.len(), size, "size mismatch at {}", i);
        for (j, &byte) in v.iter().enumerate() {
            assert_eq!(
                byte,
                (i as u8).wrapping_add(j as u8),
                "pattern mismatch at vec {} byte {}",
                i,
                j
            );
        }
    }

    drop(vecs);

    let free_after = pmm.free_frames();
    let leaked = free_before.saturating_sub(free_after);
    crate::serial_println!(
        "Stress alloc: 1000 vecs created, verified, freed. Leaked frames: {}",
        leaked
    );
    if leaked > 30 {
        crate::serial_println!("WARNING: significant frame leak detected");
    } else {
        crate::serial_println!("Stress alloc: PASS");
    }
}

fn stress_vfs() {
    use crate::fs::vfs::{InodeType, Vfs};

    crate::serial_println!("Stress: creating 100 files in /tmp...");

    let tmp = Vfs::resolve("/tmp").expect("resolve /tmp");

    // Create 100 files
    for i in 0..100 {
        let name = alloc::format!("stress_{:04}", i);
        let file = tmp
            .create(&name, InodeType::File, 0o644)
            .expect("create failed");
        let data = alloc::format!("content of file {}", i);
        file.write(0, data.as_bytes()).expect("write failed");
    }

    // Read them all back
    for i in 0..100 {
        let name = alloc::format!("stress_{:04}", i);
        let file = tmp.lookup(&name).expect("lookup failed");
        let mut buf = [0u8; 64];
        let n = file.read(0, &mut buf).expect("read failed");
        let expected = alloc::format!("content of file {}", i);
        assert_eq!(
            &buf[..n],
            expected.as_bytes(),
            "content mismatch for file {}",
            i
        );
    }

    // Delete them all
    for i in 0..100 {
        let name = alloc::format!("stress_{:04}", i);
        tmp.unlink(&name).expect("unlink failed");
    }

    // Verify they're gone
    let entries = tmp.readdir().expect("readdir");
    let remaining = entries
        .iter()
        .filter(|e| e.name.starts_with("stress_"))
        .count();
    assert_eq!(remaining, 0, "files not cleaned up");

    crate::serial_println!("Stress VFS: 100 files created, read, verified, deleted. PASS");
}

fn stress_pipe() {
    use crate::fs::vfs::Inode;
    use crate::ipc::pipe::Pipe;

    crate::serial_println!("Stress: pipe throughput...");

    let (reader, writer) = Pipe::create();
    let total_bytes: usize = 256 * 1024; // 256 KiB
    let mut written_total = 0;
    let mut read_total = 0;

    // Write and read in alternating chunks
    let write_data = [0xABu8; 4096];
    let mut read_buf = [0u8; 4096];

    while written_total < total_bytes {
        let w = writer.write(0, &write_data).expect("pipe write");
        written_total += w;

        let r = reader.read(0, &mut read_buf).expect("pipe read");
        read_total += r;
        assert!(read_buf[..r].iter().all(|&b| b == 0xAB), "data corruption");
    }

    // Drain remaining
    drop(writer);
    loop {
        let r = reader.read(0, &mut read_buf).expect("pipe drain");
        if r == 0 {
            break;
        }
        read_total += r;
    }

    assert_eq!(written_total, read_total, "byte count mismatch");
    crate::serial_println!("Stress pipe: {} KiB transferred. PASS", read_total / 1024);
}

pub fn bench(args: &[String]) {
    let test = if args.is_empty() { "all" } else { &args[0] };

    match test {
        "pmm" => bench_pmm(),
        "slab" => bench_slab(),
        "pipe" => bench_pipe(),
        "all" => {
            bench_pmm();
            bench_slab();
            bench_pipe();
        }
        _ => crate::serial_println!("bench: unknown test. Try: pmm, slab, pipe, all"),
    }
}

fn bench_pmm() {
    let pmm = crate::memory::pmm::Pmm::get();
    let iterations = 10000u64;

    let t0 = crate::arch::x86_64::apic::ticks();
    for _ in 0..iterations {
        let frame = pmm.alloc().expect("bench: PMM alloc failed");
        // SAFETY: Frame was just allocated.
        unsafe { pmm.dealloc(frame) };
    }
    let t1 = crate::arch::x86_64::apic::ticks();
    let elapsed_ms = t1 - t0;
    let ops_per_sec = (iterations * 1000).checked_div(elapsed_ms).unwrap_or(0);
    crate::serial_println!(
        "Bench PMM: {} alloc+dealloc in {} ms ({} ops/sec)",
        iterations,
        elapsed_ms,
        ops_per_sec
    );
}

fn bench_slab() {
    extern crate alloc;
    use alloc::boxed::Box;

    let iterations = 10000u64;

    let t0 = crate::arch::x86_64::apic::ticks();
    for _ in 0..iterations {
        let b = Box::new(42u64);
        core::hint::black_box(&b);
        drop(b);
    }
    let t1 = crate::arch::x86_64::apic::ticks();
    let elapsed_ms = t1 - t0;
    let ops_per_sec = (iterations * 1000).checked_div(elapsed_ms).unwrap_or(0);
    crate::serial_println!(
        "Bench slab: {} Box alloc+drop in {} ms ({} ops/sec)",
        iterations,
        elapsed_ms,
        ops_per_sec
    );
}

fn bench_pipe() {
    use crate::fs::vfs::Inode;
    use crate::ipc::pipe::Pipe;

    let (reader, writer) = Pipe::create();
    let data = [0xABu8; 4096];
    let mut buf = [0u8; 4096];
    let iterations = 1000u64;

    let t0 = crate::arch::x86_64::apic::ticks();
    for _ in 0..iterations {
        writer.write(0, &data).expect("bench: pipe write");
        reader.read(0, &mut buf).expect("bench: pipe read");
    }
    let t1 = crate::arch::x86_64::apic::ticks();
    let elapsed_ms = t1 - t0;
    let total_bytes = iterations * 4096;
    let throughput_mb = (total_bytes * 1000)
        .checked_div(elapsed_ms * 1024 * 1024)
        .unwrap_or(0);
    crate::serial_println!(
        "Bench pipe: {} x 4K round-trips in {} ms ({} MiB/sec)",
        iterations,
        elapsed_ms,
        throughput_mb
    );
}

pub fn leak_check() {
    let pmm = crate::memory::pmm::Pmm::get();
    let free_before = pmm.free_frames();

    crate::serial_println!("Leak check: running allocation workload...");

    // Workload: allocate and free various sizes
    {
        extern crate alloc;
        use alloc::collections::BTreeMap;
        use alloc::string::String;
        use alloc::vec::Vec;

        let mut vecs: Vec<Vec<u8>> = Vec::new();
        for i in 0..500 {
            vecs.push(alloc::vec![i as u8; 64 + (i % 16) * 64]);
        }
        drop(vecs);

        let mut map: BTreeMap<u32, String> = BTreeMap::new();
        for i in 0..200 {
            map.insert(i, alloc::format!("value_{}", i));
        }
        drop(map);

        // Pipe workload
        use crate::fs::vfs::Inode;
        use crate::ipc::pipe::Pipe;
        let (r, w) = Pipe::create();
        for _ in 0..100 {
            w.write(0, &[0u8; 1024]).unwrap();
            let mut buf = [0u8; 1024];
            r.read(0, &mut buf).unwrap();
        }
        drop(w);
        drop(r);
    }

    let free_after = pmm.free_frames();
    let leaked = free_before.saturating_sub(free_after);

    crate::serial_println!(
        "Leak check: frames before={}, after={}, delta={}",
        free_before,
        free_after,
        leaked
    );
    if leaked <= 5 {
        crate::serial_println!("Leak check: PASS (no significant leaks)");
    } else {
        crate::serial_println!("Leak check: WARNING — {} frames potentially leaked", leaked);
    }
}
