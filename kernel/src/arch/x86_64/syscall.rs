use crate::arch::x86_64::gdt;

const IA32_STAR: u32 = 0xC000_0081;
const IA32_LSTAR: u32 = 0xC000_0082;
const IA32_SFMASK: u32 = 0xC000_0084;
const IA32_EFER: u32 = 0xC000_0080;

const EFER_SCE: u64 = 1 << 0;
const SFMASK: u64 = 0x200 | 0x100 | 0x400; // IF | TF | DF

/// Initialize SYSCALL/SYSRET MSRs.
///
/// # Safety
/// Must be called once during boot after the GDT is loaded.
pub unsafe fn init() {
    // SAFETY: IA32_EFER exists on all x86_64 CPUs.
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | EFER_SCE);
    }

    // STAR[47:32] = kernel CS for SYSCALL
    // STAR[63:48] = base for SYSRET (adds +16 for CS, +8 for SS)
    // GDT: 0x08=kCS, 0x10=kDS, 0x18=uDS, 0x20=uCS
    // SYSRET needs CS=0x20 → base+16=0x20 → base=0x10
    // SYSRET needs SS=0x18 → base+8=0x18 → base=0x10
    let kernel_cs = (gdt::KERNEL_CS & !3) as u64;
    let star = (0x10u64 << 48) | (kernel_cs << 32);

    // SAFETY: Setting SYSCALL/SYSRET MSRs with valid segment selectors.
    unsafe {
        wrmsr(IA32_STAR, star);
        wrmsr(IA32_LSTAR, syscall_entry as *const () as u64);
        wrmsr(IA32_SFMASK, SFMASK);
    }

    crate::serial_println!("SYSCALL: MSRs configured");
}

/// SYSCALL entry point.
///
/// On entry: RCX=user RIP, R11=user RFLAGS, RAX=syscall number.
/// Args: RDI, RSI, RDX, R10, R8, R9.
#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // SWAPGS to get kernel GS base (per-CPU data)
        "swapgs",

        // Save user RSP and load kernel RSP
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        // Save all registers we need to preserve
        "push rcx",        // user RIP
        "push r11",        // user RFLAGS
        "push gs:[0]",     // user RSP
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Set up arguments for dispatch(num, a1, a2, a3, a4, a5, a6)
        // Currently: rax=num, rdi=a1, rsi=a2, rdx=a3, r10=a4, r8=a5, r9=a6
        // System V calling convention: rdi, rsi, rdx, rcx, r8, r9, stack
        "push r9",         // a6 → stack
        "sub rsp, 8",     // 16-byte alignment
        "mov r9, r8",      // a5
        "mov r8, r10",     // a4
        "mov rcx, rdx",    // a3
        "mov rdx, rsi",    // a2
        "mov rsi, rdi",    // a1
        "mov rdi, rax",    // syscall number

        "call {dispatch}",

        // RAX now holds the return value
        "add rsp, 16",    // Remove alignment + a6

        // Check for pending signals before returning to user mode.
        // Pass pointer to saved user context on stack (r15..rcx).
        // RDI = pointer to saved context, RSI = syscall return value (rax)
        "mov rdi, rsp",
        "mov rsi, rax",
        "call {check_signals}",
        // RAX may be modified by signal delivery

        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        // Restore user state for SYSRET
        "pop rdx",         // user RSP → temp in rdx
        "pop r11",         // user RFLAGS
        "pop rcx",         // user RIP

        // Restore user stack
        "mov rsp, rdx",

        // SWAPGS back to user GS
        "swapgs",

        // Return to userspace
        "sysretq",

        dispatch = sym crate::syscall::table::dispatch,
        check_signals = sym check_pending_signals,
    );
}

/// Signal frame pushed on the user stack before signal delivery.
/// The handler returns via sigreturn which restores this context.
#[repr(C)]
struct SignalFrame {
    /// Sigreturn trampoline: mov rax, 15 (SYS_SIGRETURN); syscall; (12 bytes)
    trampoline: [u8; 16],
    /// Signal number
    signo: u64,
    /// Saved user RIP (return address after signal handler)
    saved_rip: u64,
    /// Saved user RSP
    saved_rsp: u64,
    /// Saved user RFLAGS
    saved_rflags: u64,
    /// Saved syscall return value (RAX)
    saved_rax: u64,
}

/// Check for pending signals and set up delivery if needed.
/// Called from syscall exit path with pointer to saved register context.
///
/// Context layout on stack (from ctx_ptr):
///   [0]=r15, [1]=r14, [2]=r13, [3]=r12, [4]=rbx, [5]=rbp,
///   [6]=user_rsp, [7]=user_rflags, [8]=user_rip
#[no_mangle]
extern "C" fn check_pending_signals(ctx_ptr: *mut u64, syscall_ret: i64) -> i64 {
    use crate::process::signal::{self, SigHandler};
    use crate::syscall::table::current_pid_value;

    let pid = current_pid_value();

    // Check if there's a deliverable signal
    let (sig, handler_addr) = match signal::with_signal_state(pid, |state| {
        if let Some(sig) = state.dequeue() {
            match state.get_handler(sig) {
                SigHandler::Handler(addr) => Some((sig, addr)),
                SigHandler::Ignore => None,
                SigHandler::Default => {
                    // Default action: terminate for most signals
                    match sig.default_action() {
                        signal::SignalAction::Terminate => {
                            crate::serial_println!(
                                "Signal {}: default terminate pid {}",
                                sig as u8,
                                pid
                            );
                            None // Will be handled by caller
                        }
                        _ => None,
                    }
                }
            }
        } else {
            None
        }
    }) {
        Some(Some(result)) => result,
        _ => return syscall_ret, // No signal to deliver
    };

    // SAFETY: ctx_ptr points to valid saved registers on kernel stack.
    unsafe {
        let user_rsp = *ctx_ptr.add(6);
        let user_rflags = *ctx_ptr.add(7);
        let user_rip = *ctx_ptr.add(8);

        // Build signal frame on user stack
        let frame_size = core::mem::size_of::<SignalFrame>() as u64;
        let new_user_rsp = (user_rsp - frame_size) & !0xF; // 16-byte align

        // Write signal frame to user stack via HHDM
        // Note: this assumes the user stack page is mapped in the current CR3
        let frame_ptr = new_user_rsp as *mut SignalFrame;

        // Sigreturn trampoline: mov rax, 15; syscall; nop padding
        let mut trampoline = [0x90u8; 16]; // NOP padding
        trampoline[0..7].copy_from_slice(&[0x48, 0xC7, 0xC0, 15, 0, 0, 0]); // mov rax, 15
        trampoline[7..9].copy_from_slice(&[0x0F, 0x05]); // syscall

        (*frame_ptr).trampoline = trampoline;
        (*frame_ptr).signo = sig as u64;
        (*frame_ptr).saved_rip = user_rip;
        (*frame_ptr).saved_rsp = user_rsp;
        (*frame_ptr).saved_rflags = user_rflags;
        (*frame_ptr).saved_rax = syscall_ret as u64;

        // Modify saved context:
        // - RIP → handler function
        // - RSP → below signal frame
        // - RDI → signal number (first arg to handler)
        *ctx_ptr.add(8) = handler_addr; // user_rip = handler
        *ctx_ptr.add(6) = new_user_rsp; // user_rsp = below frame

        // The handler's return address should be the trampoline
        // Push trampoline address on the new user stack
        let ret_addr_ptr = (new_user_rsp - 8) as *mut u64;
        *ret_addr_ptr = new_user_rsp; // Return to trampoline (start of frame)
        *ctx_ptr.add(6) = new_user_rsp - 8; // Adjust RSP for pushed return addr
    }

    syscall_ret
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let (lo, hi): (u32, u32);
    // SAFETY: Caller ensures MSR exists.
    unsafe {
        core::arch::asm!("rdmsr", in("ecx") msr, out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    (hi as u64) << 32 | lo as u64
}

unsafe fn wrmsr(msr: u32, val: u64) {
    let lo = val as u32;
    let hi = (val >> 32) as u32;
    // SAFETY: Caller ensures MSR exists and value is valid.
    unsafe {
        core::arch::asm!("wrmsr", in("ecx") msr, in("eax") lo, in("edx") hi, options(nomem, nostack));
    }
}
