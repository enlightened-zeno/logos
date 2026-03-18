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
        SYS_EXIT | SYS_EXIT_GROUP => sys_exit(a1 as i32),
        SYS_BRK => sys_brk(a1),
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

fn sys_brk(addr: u64) -> SyscallResult {
    // Stub: return current break (0 = let libc figure it out)
    let _ = addr;
    0
}
