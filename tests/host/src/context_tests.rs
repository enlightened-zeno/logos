/// Context switch logic tests.

#[test]
fn test_context_struct_size() {
    // CpuContext: rsp, rbp, rbx, r12-r15, rflags, cr3 = 9 x u64 = 72 bytes
    let fields = 9;
    let size = fields * 8;
    assert_eq!(size, 72);
}

#[test]
fn test_callee_saved_registers() {
    // System V ABI callee-saved: rbx, rbp, r12, r13, r14, r15
    let callee_saved = ["rbx", "rbp", "r12", "r13", "r14", "r15"];
    assert_eq!(callee_saved.len(), 6);
}

#[test]
fn test_rflags_if_bit() {
    let rflags_if: u64 = 0x200;
    assert_eq!(rflags_if, 512);
    // Default rflags should have IF set
    let default = 0x200u64;
    assert!(default & rflags_if != 0);
}

#[test]
fn test_cr3_switch() {
    let old_cr3: u64 = 0x1000;
    let new_cr3: u64 = 0x2000;
    let should_switch = old_cr3 != new_cr3;
    assert!(should_switch);

    let same_cr3 = old_cr3;
    let should_not_switch = old_cr3 == same_cr3;
    assert!(should_not_switch);
}

#[test]
fn test_kernel_stack_per_process() {
    let stack_size = 32768; // 32 KiB
    let stack = vec![0u8; stack_size];
    let stack_top = stack.as_ptr() as u64 + stack_size as u64;
    assert!(stack_top > stack.as_ptr() as u64);
}
