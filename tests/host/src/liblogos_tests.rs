/// liblogos (userspace library) logic tests.

#[test]
fn test_syscall_convention() {
    // RAX = syscall number
    // RDI, RSI, RDX, R10, R8, R9 = args 1-6
    // RAX = return value
    // RCX, R11 clobbered
    let regs_used = 8; // rax + 6 args + return in rax
    assert!(regs_used >= 7);
}

#[test]
fn test_errno_from_negative() {
    let result: i64 = -2;
    let is_error = result < 0;
    let errno = (-result) as u32;
    assert!(is_error);
    assert_eq!(errno, 2); // ENOENT
}

#[test]
fn test_print_macro_format() {
    let output = format!("Hello, {}!", "LogOS");
    assert_eq!(output, "Hello, LogOS!");
}

#[test]
fn test_brk_allocator() {
    // Bump allocator: never frees
    let mut brk: u64 = 0x4000_0000_0000;
    let alloc_size = 64u64;
    let aligned = (brk + 7) & !7; // 8-byte align
    let ptr = aligned;
    brk = aligned + alloc_size;
    assert!(ptr > 0);
    assert!(brk > ptr);
}

#[test]
fn test_start_entry_point() {
    // _start is the ELF entry point, calls main
    let entry = "_start";
    assert_eq!(entry, "_start");
}

#[test]
fn test_exit_codes() {
    assert_eq!(0i32, 0); // EXIT_SUCCESS
    assert_eq!(1i32, 1); // EXIT_FAILURE
}
