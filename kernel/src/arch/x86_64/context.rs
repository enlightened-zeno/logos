/// Saved CPU context for context switching.
///
/// Only callee-saved registers are stored here — the caller-saved registers
/// are already on the stack by the time we context switch (System V ABI).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CpuContext {
    pub rsp: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
    /// CR3 (page table base) — each process has its own address space.
    pub cr3: u64,
}

impl CpuContext {
    pub const fn empty() -> Self {
        Self {
            rsp: 0,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x200, // IF=1 (interrupts enabled)
            cr3: 0,
        }
    }

    /// Create a context for a new kernel task.
    ///
    /// When this context is switched to, execution starts at `entry_point`
    /// with the given stack pointer.
    pub fn new_kernel(_entry_point: u64, stack_top: u64, cr3: u64) -> Self {
        Self {
            rsp: stack_top - 8, // Space for the return address
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x200,
            cr3,
        }
    }
}

/// Switch from one CPU context to another.
///
/// Saves callee-saved registers into `old`, loads them from `new`,
/// and switches the stack. If CR3 differs, the page tables are switched too.
///
/// # Safety
/// Both contexts must be valid. The new context's stack and entry point
/// must be properly set up.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: *mut CpuContext, new: *const CpuContext) {
    core::arch::naked_asm!(
        // Save callee-saved registers into old context
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], rbp",
        "mov [rdi + 0x10], rbx",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], r13",
        "mov [rdi + 0x28], r14",
        "mov [rdi + 0x30], r15",
        "pushfq",
        "pop qword ptr [rdi + 0x38]",
        "mov rax, cr3",
        "mov [rdi + 0x40], rax",
        // Load callee-saved registers from new context
        "mov rsp, [rsi + 0x00]",
        "mov rbp, [rsi + 0x08]",
        "mov rbx, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov r13, [rsi + 0x20]",
        "mov r14, [rsi + 0x28]",
        "mov r15, [rsi + 0x30]",
        "push qword ptr [rsi + 0x38]",
        "popfq",
        // Switch page tables if CR3 changed
        "mov rax, [rsi + 0x40]",
        "mov rcx, cr3",
        "cmp rax, rcx",
        "je 2f",
        "mov cr3, rax",
        "2:",
        "ret",
    );
}
