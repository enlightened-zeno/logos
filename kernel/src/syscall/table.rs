use crate::syscall::errno::{Errno, SyscallResult};
use core::sync::atomic::{AtomicU64, Ordering};

/// Current process ID. Updated on context switch.
static CURRENT_PID: AtomicU64 = AtomicU64::new(1); // Starts as init (PID 1)

/// Get the current process PID.
fn current_pid() -> u64 {
    CURRENT_PID.load(Ordering::Relaxed)
}

/// Get the current process PID (public for exec module).
pub fn current_pid_value() -> u64 {
    current_pid()
}

/// Set the current process PID (called on context switch or exec).
pub fn set_current_pid(pid: u64) {
    CURRENT_PID.store(pid, Ordering::Relaxed);
}

/// Syscall numbers (Linux-compatible subset).
pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_OPEN: u64 = 2;
pub const SYS_CLOSE: u64 = 3;
pub const SYS_STAT: u64 = 4;
pub const SYS_FSTAT: u64 = 5;
pub const SYS_LSEEK: u64 = 8;
pub const SYS_MMAP: u64 = 9;
pub const SYS_MPROTECT: u64 = 10;
pub const SYS_MUNMAP: u64 = 11;
pub const SYS_BRK: u64 = 12;
pub const SYS_IOCTL: u64 = 16;
pub const SYS_PIPE: u64 = 22;
pub const SYS_DUP: u64 = 32;
pub const SYS_DUP2: u64 = 33;
pub const SYS_NANOSLEEP: u64 = 35;
pub const SYS_GETPID: u64 = 39;
pub const SYS_FORK: u64 = 57;
pub const SYS_EXECVE: u64 = 59;
pub const SYS_EXIT: u64 = 60;
pub const SYS_WAIT4: u64 = 61;
pub const SYS_KILL: u64 = 62;
pub const SYS_UNAME: u64 = 63;
pub const SYS_GETUID: u64 = 102;
pub const SYS_GETGID: u64 = 104;
pub const SYS_GETEUID: u64 = 107;
pub const SYS_GETEGID: u64 = 108;
pub const SYS_SETPGID: u64 = 109;
pub const SYS_GETPPID: u64 = 110;
pub const SYS_SETSID: u64 = 112;
pub const SYS_SIGACTION: u64 = 13;
pub const SYS_SIGPROCMASK: u64 = 14;
pub const SYS_SIGRETURN: u64 = 15;
pub const SYS_CLOCK_GETTIME: u64 = 228;
pub const SYS_EXIT_GROUP: u64 = 231;
pub const SYS_MKDIR: u64 = 83;
pub const SYS_RMDIR: u64 = 84;
pub const SYS_UNLINK: u64 = 87;
pub const SYS_GETCWD: u64 = 79;
pub const SYS_CHDIR: u64 = 80;
pub const SYS_GETDENTS64: u64 = 217;
pub const SYS_SHMGET: u64 = 29;
pub const SYS_SHMAT: u64 = 30;
pub const SYS_SHMDT: u64 = 67;

/// Dispatch a syscall by number.
///
/// Arguments follow the System V ABI syscall convention:
/// - rax: syscall number
/// - rdi, rsi, rdx, r10, r8, r9: arguments 1-6
pub fn dispatch(
    num: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> SyscallResult {
    match num {
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_GETUID | SYS_GETEUID => sys_getuid(),
        SYS_GETGID | SYS_GETEGID => sys_getgid(),
        SYS_UNAME => sys_uname(a1),
        SYS_WRITE => sys_write(a1, a2, a3),
        SYS_READ => sys_read(a1, a2, a3),
        SYS_EXIT | SYS_EXIT_GROUP => sys_exit(a1 as i32),
        SYS_BRK => sys_brk(a1),
        SYS_NANOSLEEP => sys_nanosleep(a1),
        SYS_CLOCK_GETTIME => sys_clock_gettime(a1, a2),
        SYS_OPEN => sys_open(a1, a2, a3),
        SYS_CLOSE => sys_close(a1),
        SYS_STAT => sys_stat(a1, a2),
        SYS_FSTAT => sys_fstat(a1, a2),
        SYS_LSEEK => sys_lseek(a1, a2, a3),
        SYS_PIPE => sys_pipe(a1),
        SYS_DUP => sys_dup(a1),
        SYS_DUP2 => sys_dup2(a1, a2),
        SYS_MKDIR => sys_mkdir(a1, a2),
        SYS_RMDIR => sys_rmdir(a1),
        SYS_UNLINK => sys_unlink(a1),
        SYS_GETCWD => sys_getcwd(a1, a2),
        SYS_CHDIR => sys_chdir(a1),
        SYS_GETDENTS64 => sys_getdents64(a1, a2, a3),
        SYS_KILL => sys_kill(a1, a2),
        SYS_SIGACTION => sys_sigaction(a1, a2),
        SYS_SIGPROCMASK => sys_sigprocmask(a1, a2, a3),
        SYS_SIGRETURN => sys_sigreturn(),
        SYS_SETPGID => sys_setpgid(a1, a2),
        SYS_SETSID => sys_setsid(),
        SYS_FORK => sys_fork(),
        SYS_WAIT4 => sys_wait4(a1, a2, a3),
        SYS_EXECVE => sys_execve(a1, a2, a3),
        _ => {
            crate::serial_println!("syscall: unimplemented #{}", num);
            Errno::ENOSYS.as_neg()
        }
    }
}

// Stub implementations — these will be replaced as subsystems come online.

fn sys_getpid() -> SyscallResult {
    current_pid() as i64
}

fn sys_getppid() -> SyscallResult {
    crate::process::pid::get_ppid(current_pid()).unwrap_or(0) as i64
}

fn sys_getuid() -> SyscallResult {
    0 // root
}

fn sys_getgid() -> SyscallResult {
    0 // root
}

fn sys_uname(buf_ptr: u64) -> SyscallResult {
    use crate::syscall::validate;

    // struct utsname: 5 fields of 65 bytes each
    const FIELD_LEN: usize = 65;
    let mut buf = [0u8; FIELD_LEN * 6];

    let sysname = b"LogOS";
    let nodename = b"logos";
    let release = b"0.1.0";
    let version = b"#1";
    let machine = b"x86_64";

    buf[..sysname.len()].copy_from_slice(sysname);
    buf[FIELD_LEN..FIELD_LEN + nodename.len()].copy_from_slice(nodename);
    buf[FIELD_LEN * 2..FIELD_LEN * 2 + release.len()].copy_from_slice(release);
    buf[FIELD_LEN * 3..FIELD_LEN * 3 + version.len()].copy_from_slice(version);
    buf[FIELD_LEN * 4..FIELD_LEN * 4 + machine.len()].copy_from_slice(machine);

    match validate::copy_to_user(buf_ptr, &buf) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_write(fd: u64, buf_ptr: u64, count: u64) -> SyscallResult {
    use crate::syscall::validate;

    if count == 0 {
        return 0;
    }
    if count > 4096 {
        return Errno::EINVAL.as_neg();
    }

    let mut kbuf = [0u8; 4096];
    let slice = &mut kbuf[..count as usize];
    if let Err(e) = validate::copy_from_user(buf_ptr, slice) {
        return e.as_neg();
    }

    // Try per-process FD table first
    let pid = current_pid();
    if let Ok(n) = crate::fs::fd::with_fd_table(pid, |table| {
        let fd_entry = table.get(fd as usize)?;
        if fd_entry.flags.write {
            fd_entry.inode.write(fd_entry.offset, slice)
        } else {
            Err(Errno::EBADF)
        }
    }) {
        return n as i64;
    }

    // Fallback: direct serial output for stdout/stderr
    match fd {
        1 | 2 => {
            for &byte in slice.iter() {
                crate::drivers::serial::write_byte(byte);
            }
            count as i64
        }
        _ => Errno::EBADF.as_neg(),
    }
}

fn sys_exit(code: i32) -> SyscallResult {
    let pid = current_pid();

    // Close all open file descriptors and signal state (but not for PID 1 / init)
    if pid != 1 {
        crate::fs::fd::remove_for_pid(pid);
        crate::process::signal::remove_for_pid(pid);
    }

    // Reparent children to init
    crate::process::pid::reparent_children(pid);

    // Mark self as zombie
    crate::process::pid::set_zombie(pid, code);

    // Return to kernel context (restores kernel CR3/RSP and returns
    // from run_user_program with the exit code)
    crate::process::exec::return_to_kernel(code);
}

fn sys_read(fd: u64, buf_ptr: u64, count: u64) -> SyscallResult {
    use crate::syscall::validate;

    if count == 0 {
        return 0;
    }
    if count > 4096 {
        return Errno::EINVAL.as_neg();
    }

    let mut kbuf = [0u8; 4096];
    let n = count as usize;

    // Try per-process FD table
    let pid = current_pid();
    if let Ok(bytes_read) = crate::fs::fd::with_fd_table(pid, |table| {
        let fd_entry = table.get(fd as usize)?;
        if fd_entry.flags.read {
            fd_entry.inode.read(fd_entry.offset, &mut kbuf[..n])
        } else {
            Err(Errno::EBADF)
        }
    }) {
        if bytes_read > 0 {
            if let Err(e) = validate::copy_to_user(buf_ptr, &kbuf[..bytes_read]) {
                return e.as_neg();
            }
        }
        return bytes_read as i64;
    }

    // Fallback: stdin from TTY
    match fd {
        0 => {
            let nr = crate::tty::read(&mut kbuf[..n]);
            if nr > 0 {
                if let Err(e) = validate::copy_to_user(buf_ptr, &kbuf[..nr]) {
                    return e.as_neg();
                }
            }
            nr as i64
        }
        _ => Errno::EBADF.as_neg(),
    }
}

fn sys_brk(addr: u64) -> SyscallResult {
    let _ = addr;
    0
}

fn sys_nanosleep(req_ptr: u64) -> SyscallResult {
    // struct timespec { tv_sec: i64, tv_nsec: i64 }
    // For simplicity, just read the first 8 bytes as seconds
    if req_ptr == 0 {
        return Errno::EFAULT.as_neg();
    }

    // Read seconds from user space (simplified: assume kernel pointer for now)
    let ms = 100u64; // Default 100ms if we can't read the pointer
    crate::timer::sleep_ms(ms);
    0
}

fn sys_clock_gettime(clock_id: u64, tp_ptr: u64) -> SyscallResult {
    use crate::syscall::validate;

    let _ = clock_id; // We only have CLOCK_MONOTONIC
    let ticks = crate::arch::x86_64::apic::ticks();
    let secs = ticks / 1000;
    let nsecs = (ticks % 1000) * 1_000_000;

    // struct timespec { tv_sec: i64, tv_nsec: i64 }
    let mut buf = [0u8; 16];
    buf[..8].copy_from_slice(&(secs as i64).to_le_bytes());
    buf[8..16].copy_from_slice(&(nsecs as i64).to_le_bytes());

    match validate::copy_to_user(tp_ptr, &buf) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_pipe(fds_ptr: u64) -> SyscallResult {
    // Would create a pipe and return two FDs
    // Stub for now — returns ENOSYS until we have per-process FD tables
    let _ = fds_ptr;
    Errno::ENOSYS.as_neg()
}

fn sys_dup(fd: u64) -> SyscallResult {
    let pid = current_pid();
    match crate::fs::fd::with_fd_table(pid, |table| table.dup(fd as usize)) {
        Ok(new_fd) => new_fd as i64,
        Err(e) => e.as_neg(),
    }
}

fn sys_dup2(old_fd: u64, new_fd: u64) -> SyscallResult {
    let pid = current_pid();
    match crate::fs::fd::with_fd_table(pid, |table| table.dup2(old_fd as usize, new_fd as usize)) {
        Ok(fd) => fd as i64,
        Err(e) => e.as_neg(),
    }
}

fn sys_close(fd: u64) -> SyscallResult {
    let pid = current_pid();
    match crate::fs::fd::with_fd_table(pid, |table| table.close(fd as usize)) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_open(path_ptr: u64, flags: u64, mode: u64) -> SyscallResult {
    use crate::fs::fd::OpenFlags;
    use crate::fs::vfs::Vfs;
    use crate::syscall::validate;
    let _ = mode;

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    let inode = match Vfs::resolve(&path) {
        Ok(i) => i,
        Err(e) => return e.as_neg(),
    };

    // Translate flags (O_RDONLY=0, O_WRONLY=1, O_RDWR=2)
    let open_flags = match flags & 3 {
        0 => OpenFlags::RDONLY,
        1 => OpenFlags::WRONLY,
        2 => OpenFlags::RDWR,
        _ => OpenFlags::RDONLY,
    };

    let pid = current_pid();
    match crate::fs::fd::with_fd_table(pid, |table| table.alloc(inode, open_flags)) {
        Ok(fd) => fd as i64,
        Err(e) => e.as_neg(),
    }
}

fn sys_stat(path_ptr: u64, buf_ptr: u64) -> SyscallResult {
    let _ = (path_ptr, buf_ptr);
    Errno::ENOSYS.as_neg()
}

fn sys_fstat(fd: u64, buf_ptr: u64) -> SyscallResult {
    let _ = (fd, buf_ptr);
    Errno::ENOSYS.as_neg()
}

fn sys_lseek(fd: u64, offset: u64, whence: u64) -> SyscallResult {
    let _ = (fd, offset, whence);
    Errno::ENOSYS.as_neg()
}

fn sys_mkdir(path_ptr: u64, mode: u64) -> SyscallResult {
    use crate::syscall::validate;
    let _ = mode;

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    match crate::fs::vfs::Vfs::resolve(&path).map(|_| ()) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_rmdir(path_ptr: u64) -> SyscallResult {
    use crate::syscall::validate;

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    match crate::fs::vfs::Vfs::resolve(&path).map(|_| ()) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_unlink(path_ptr: u64) -> SyscallResult {
    use crate::syscall::validate;

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    match crate::fs::vfs::Vfs::resolve(&path).map(|_| ()) {
        Ok(()) => 0,
        Err(e) => e.as_neg(),
    }
}

fn sys_getcwd(buf_ptr: u64, size: u64) -> SyscallResult {
    use crate::syscall::validate;

    let cwd = b"/\0";
    if size < cwd.len() as u64 {
        return Errno::ERANGE.as_neg();
    }
    match validate::copy_to_user(buf_ptr, cwd) {
        Ok(()) => buf_ptr as i64,
        Err(e) => e.as_neg(),
    }
}

fn sys_chdir(path_ptr: u64) -> SyscallResult {
    use crate::syscall::validate;

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    // Verify the path exists
    match crate::fs::vfs::Vfs::resolve(&path).ok() {
        Some(_) => 0,
        None => Errno::ENOENT.as_neg(),
    }
}

fn sys_getdents64(fd: u64, buf_ptr: u64, count: u64) -> SyscallResult {
    let _ = (fd, buf_ptr, count);
    Errno::ENOSYS.as_neg()
}

fn sys_kill(pid: u64, sig: u64) -> SyscallResult {
    use crate::process::signal::{self, Signal};

    let sig = match Signal::from_number(sig as u8) {
        Some(s) => s,
        None => return Errno::EINVAL.as_neg(),
    };

    let target = if pid == 0 { current_pid() } else { pid };

    if signal::send_signal(target, sig) {
        0
    } else {
        Errno::ESRCH.as_neg()
    }
}

fn sys_sigaction(sig: u64, handler_ptr: u64) -> SyscallResult {
    use crate::process::signal::{self, SigHandler, Signal};

    let sig = match Signal::from_number(sig as u8) {
        Some(s) => s,
        None => return Errno::EINVAL.as_neg(),
    };

    // SIGKILL and SIGSTOP cannot be caught
    if sig == Signal::SIGKILL || sig == Signal::SIGSTOP {
        return Errno::EINVAL.as_neg();
    }

    let handler = if handler_ptr == 0 {
        SigHandler::Default
    } else if handler_ptr == 1 {
        SigHandler::Ignore // SIG_IGN
    } else {
        SigHandler::Handler(handler_ptr)
    };

    let pid = current_pid();
    match signal::with_signal_state(pid, |state| {
        state.set_handler(sig, handler);
    }) {
        Some(()) => 0,
        None => Errno::ESRCH.as_neg(),
    }
}

fn sys_sigprocmask(how: u64, set: u64, _oldset: u64) -> SyscallResult {
    use crate::process::signal;

    let pid = current_pid();
    match signal::with_signal_state(pid, |state| {
        match how {
            0 => state.blocked |= set,  // SIG_BLOCK
            1 => state.blocked &= !set, // SIG_UNBLOCK
            2 => state.blocked = set,   // SIG_SETMASK
            _ => {}
        }
    }) {
        Some(()) => 0,
        None => Errno::ESRCH.as_neg(),
    }
}

fn sys_sigreturn() -> SyscallResult {
    // In a full implementation, this would restore the user context from
    // the signal frame on the user stack. For now, it's a no-op since
    // we handle signal delivery synchronously in the kernel.
    0
}

fn sys_setpgid(pid: u64, pgid: u64) -> SyscallResult {
    let _ = (pid, pgid);
    0
}

fn sys_setsid() -> SyscallResult {
    // Would create a new session
    sys_getpid()
}

fn sys_fork() -> SyscallResult {
    use crate::process::{address_space::AddressSpace, pid};

    let parent_pid = current_pid();
    let hhdm_offset = crate::memory::pmm::Pmm::get().hhdm_offset();

    // Get the current address space's CR3
    let current_cr3: u64;
    // SAFETY: Reading CR3 is always safe.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) current_cr3, options(nomem, nostack));
    }

    // Create the parent's AddressSpace wrapper (for fork)
    let parent_as = AddressSpace {
        pml4_frame: crate::memory::addr::PhysFrame::containing_address(
            crate::memory::addr::PhysAddr::new(current_cr3 & 0x000F_FFFF_FFFF_F000),
        ),
        brk: 0,
        hhdm_offset,
        stack_top: crate::process::address_space::default_stack_top(),
        heap_start: 0,
    };

    // Fork the address space (COW clone)
    let child_as = match parent_as.fork() {
        Ok(a) => a,
        Err(e) => {
            // Don't drop parent_as — it doesn't own the current page tables
            core::mem::forget(parent_as);
            return e.as_neg();
        }
    };

    // Don't drop parent_as — it doesn't own the current page tables
    core::mem::forget(parent_as);

    // Allocate a new PID
    let child_pid = pid::alloc_pid();

    // Register the child process
    pid::register(pid::ProcessDesc {
        pid: child_pid,
        ppid: parent_pid,
        pgid: parent_pid, // Inherit parent's process group
        sid: parent_pid,
        state: pid::ProcessState::Running,
        exit_code: 0,
        uid: 0,
        gid: 0,
    });

    // Create FD table for the child process
    crate::fs::fd::create_for_pid(child_pid);

    // Store child's address space CR3 for later context switch
    let _child_cr3 = child_as.cr3();
    core::mem::forget(child_as);

    // Return child PID to parent (child would get 0 in a real fork)
    child_pid as i64
}

fn sys_wait4(target_pid: u64, status_ptr: u64, options: u64) -> SyscallResult {
    use crate::process::pid;
    use crate::syscall::validate;

    let _ = options;
    let parent = current_pid();

    // Check if parent has children at all
    if !pid::has_children(parent) {
        return Errno::ECHILD.as_neg();
    }

    // Try to find a zombie child
    match pid::find_zombie_child(parent, target_pid) {
        Some((child_pid, exit_code)) => {
            // Reap the zombie
            pid::reap(child_pid);

            // Write status to user if pointer provided
            if status_ptr != 0 {
                let status = (exit_code & 0xFF) << 8; // WEXITSTATUS encoding
                let _ = validate::copy_to_user(status_ptr, &status.to_le_bytes());
            }

            child_pid as i64
        }
        None => {
            // No zombie child yet — would block in a real implementation
            // For now, return ECHILD
            Errno::ECHILD.as_neg()
        }
    }
}

fn sys_execve(path_ptr: u64, argv_ptr: u64, envp_ptr: u64) -> SyscallResult {
    extern crate alloc;
    use crate::fs::vfs::Vfs;
    use crate::syscall::validate;

    let _ = (argv_ptr, envp_ptr); // TODO: pass argc/argv/envp to user stack

    // Copy path from user space
    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    // Look up the file in VFS
    let inode = match Vfs::resolve(&path) {
        Ok(i) => i,
        Err(e) => return e.as_neg(),
    };

    // Read the file contents
    let mut elf_data = alloc::vec![0u8; 1024 * 1024]; // 1 MiB max
    let size = match inode.read(0, &mut elf_data) {
        Ok(n) => n,
        Err(e) => return e.as_neg(),
    };
    elf_data.truncate(size);

    // Get the HHDM offset
    let hhdm_offset = crate::memory::pmm::Pmm::get().hhdm_offset();

    // exec_elf never returns — it jumps to user mode
    crate::process::exec::exec_elf(&elf_data, hhdm_offset);
}
