/// Raw syscall with 0-6 arguments.

#[inline(always)]
pub fn syscall0(num: u64) -> i64 {
    let ret: i64;
    // SAFETY: Performs a syscall to the kernel via the SYSCALL instruction.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") num as i64 => ret,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall1(num: u64, a1: u64) -> i64 {
    let ret: i64;
    // SAFETY: Performs a syscall with 1 argument.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") num as i64 => ret,
            in("rdi") a1,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall2(num: u64, a1: u64, a2: u64) -> i64 {
    let ret: i64;
    // SAFETY: Performs a syscall with 2 arguments.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") num as i64 => ret,
            in("rdi") a1,
            in("rsi") a2,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall3(num: u64, a1: u64, a2: u64, a3: u64) -> i64 {
    let ret: i64;
    // SAFETY: Performs a syscall with 3 arguments.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") num as i64 => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

// Wrapper functions

pub fn exit(code: i32) -> ! {
    syscall1(super::nr::SYS_EXIT, code as u64);
    loop {}
}

pub fn write(fd: u64, buf: &[u8]) -> i64 {
    syscall3(super::nr::SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
}

pub fn read(fd: u64, buf: &mut [u8]) -> i64 {
    syscall3(super::nr::SYS_READ, fd, buf.as_mut_ptr() as u64, buf.len() as u64)
}

pub fn getpid() -> i64 {
    syscall0(super::nr::SYS_GETPID)
}

pub fn brk(addr: u64) -> i64 {
    syscall1(super::nr::SYS_BRK, addr)
}
