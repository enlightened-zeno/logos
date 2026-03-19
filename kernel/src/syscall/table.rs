use crate::syscall::errno::{Errno, SyscallResult};

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
    crate::sched::current_task_id().0 as i64
}

fn sys_getppid() -> SyscallResult {
    1 // init is always parent for now
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

    if count > 4096 {
        return Errno::EINVAL.as_neg();
    }

    let mut kbuf = [0u8; 4096];
    let slice = &mut kbuf[..count as usize];
    if let Err(e) = validate::copy_from_user(buf_ptr, slice) {
        return e.as_neg();
    }

    match fd {
        1 | 2 => {
            // stdout/stderr → serial
            for &byte in slice.iter() {
                crate::drivers::serial::write_byte(byte);
            }
            count as i64
        }
        _ => Errno::EBADF.as_neg(),
    }
}

fn sys_exit(code: i32) -> SyscallResult {
    crate::serial_println!("Process {} exited with code {}", sys_getpid(), code);
    // For now, halt
    loop {
        crate::arch::x86_64::cpu::hlt();
    }
}

fn sys_read(fd: u64, buf_ptr: u64, count: u64) -> SyscallResult {
    use crate::syscall::validate;

    if count == 0 {
        return 0;
    }
    if count > 4096 {
        return Errno::EINVAL.as_neg();
    }

    match fd {
        0 => {
            // stdin: read from TTY
            let mut kbuf = [0u8; 4096];
            let n = crate::tty::read(&mut kbuf[..count as usize]);
            if n > 0 {
                if let Err(e) = validate::copy_to_user(buf_ptr, &kbuf[..n]) {
                    return e.as_neg();
                }
            }
            n as i64
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
    let _ = fd;
    Errno::ENOSYS.as_neg()
}

fn sys_dup2(old_fd: u64, new_fd: u64) -> SyscallResult {
    let _ = (old_fd, new_fd);
    Errno::ENOSYS.as_neg()
}

fn sys_close(fd: u64) -> SyscallResult {
    let _ = fd;
    Errno::ENOSYS.as_neg()
}

fn sys_open(path_ptr: u64, flags: u64, mode: u64) -> SyscallResult {
    use crate::syscall::validate;
    let _ = (flags, mode);

    let path = match validate::copy_str_from_user(path_ptr, 256) {
        Ok(p) => p,
        Err(e) => return e.as_neg(),
    };

    // Use VFS to look up the file
    match crate::fs::vfs::Vfs::resolve(&path).ok() {
        Some(_inode) => {
            // Would allocate an FD — return 3 as placeholder (0=stdin, 1=stdout, 2=stderr)
            3
        }
        None => Errno::ENOENT.as_neg(),
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
    let _ = (pid, sig);
    // Would deliver a signal to the target process
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
    // Would clone the current process with COW
    Errno::ENOSYS.as_neg()
}

fn sys_wait4(pid: u64, status_ptr: u64, options: u64) -> SyscallResult {
    let _ = (pid, status_ptr, options);
    Errno::ECHILD.as_neg()
}

fn sys_execve(path_ptr: u64, argv_ptr: u64, envp_ptr: u64) -> SyscallResult {
    let _ = (path_ptr, argv_ptr, envp_ptr);
    Errno::ENOSYS.as_neg()
}
