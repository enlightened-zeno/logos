use crate::arch::x86_64::gdt;
use crate::memory::paging::PageFlags;
use crate::process::address_space::{AddressSpace, USER_STACK_TOP};
use crate::process::elf;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

/// Saved kernel state for returning from user mode.
/// These are accessed from naked asm, so they must be plain statics.
static mut SAVED_RSP: u64 = 0;
static SAVED_CR3: AtomicU64 = AtomicU64::new(0);
static USER_EXIT_CODE: AtomicI32 = AtomicI32::new(-1);

/// Called by sys_exit to return to kernel after user program finishes.
///
/// Restores kernel CR3 and stack pointer, then pops callee-saved registers
/// that were saved by enter_user_mode, returning execution to run_user_program.
pub fn return_to_kernel(exit_code: i32) -> ! {
    USER_EXIT_CODE.store(exit_code, Ordering::SeqCst);

    let cr3 = SAVED_CR3.load(Ordering::SeqCst);
    // SAFETY: SAVED_RSP is written only by enter_user_mode (single-threaded use).
    let rsp = unsafe { SAVED_RSP };

    // SAFETY: Restoring previously saved valid kernel context.
    // The syscall entry did swapgs (kernel GS now active). Since we're
    // bypassing sysret, we need to swapgs back so the next syscall entry
    // swaps correctly.
    // The stack has: rbp, rbx, r12, r13, r14, r15, return_addr
    // Popping them and doing ret brings us back to run_user_program.
    // SAFETY: cr3 and rsp were saved from a valid kernel context.
    unsafe {
        core::arch::asm!(
            "swapgs",          // Restore user GS (undo syscall entry's swapgs)
            "mov cr3, {cr3}",
            "mov rsp, {rsp}",
            "pop rbp",
            "pop rbx",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            "ret",
            cr3 = in(reg) cr3,
            rsp = in(reg) rsp,
            options(noreturn)
        );
    }
}

/// Enter user mode. Saves kernel context, switches to user page tables,
/// and IRETs to ring 3. When the user calls exit(), return_to_kernel
/// restores this context and returns from this function.
///
/// # Safety
/// cr3 must point to a valid PML4 with kernel mappings.
/// Arguments: rdi=entry, rsi=user_rsp, rdx=cr3 (System V ABI)
#[unsafe(naked)]
unsafe extern "C" fn enter_user_mode(_entry: u64, _user_rsp: u64, _cr3: u64) {
    core::arch::naked_asm!(
        // Save callee-saved registers
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push rbx",
        "push rbp",
        // Save RSP for return_to_kernel
        "mov [{saved_rsp}], rsp",
        // Switch to user page tables (rdx = cr3)
        "mov cr3, rdx",
        // Build IRETQ frame
        // SS = USER_DS
        "mov rax, {ss}",
        "push rax",
        // RSP = user_rsp (rsi)
        "push rsi",
        // RFLAGS = IF=1
        "mov rax, 0x202",
        "push rax",
        // CS = USER_CS
        "mov rax, {cs}",
        "push rax",
        // RIP = entry (rdi)
        "push rdi",
        "iretq",
        saved_rsp = sym SAVED_RSP,
        ss = const gdt::USER_DS,
        cs = const gdt::USER_CS,
    );
}

/// Load an ELF binary and run it in user mode.
/// Returns the exit code when the user program calls exit().
pub fn run_user_program(elf_data: &[u8], hhdm_offset: u64) -> i32 {
    let info = elf::parse(elf_data).expect("exec: invalid ELF");

    let addr_space = AddressSpace::new(hhdm_offset).expect("exec: address space");

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
                &elf_data[seg.offset as usize..(seg.offset + seg.filesz) as usize],
                seg.memsz,
                flags,
            )
            .expect("exec: load segment");
    }

    addr_space.map_user_stack().expect("exec: user stack");

    // Allocate a PID for this user process
    let child_pid = crate::process::pid::alloc_pid();
    crate::process::pid::register(crate::process::pid::ProcessDesc {
        pid: child_pid,
        ppid: 1,
        pgid: child_pid,
        sid: child_pid,
        state: crate::process::pid::ProcessState::Running,
        exit_code: 0,
        uid: 0,
        gid: 0,
    });
    crate::fs::fd::create_for_pid(child_pid);
    crate::process::signal::create_for_pid(child_pid);

    // Set current PID to the child
    let saved_pid = crate::syscall::table::current_pid_value();
    crate::syscall::table::set_current_pid(child_pid);

    // Save kernel CR3
    let current_cr3: u64;
    // SAFETY: Reading CR3 is always safe.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) current_cr3, options(nomem, nostack));
    }
    SAVED_CR3.store(current_cr3, Ordering::SeqCst);
    USER_EXIT_CODE.store(-1, Ordering::SeqCst);

    // Enter user mode — this "returns" when return_to_kernel is called
    // SAFETY: addr_space has valid kernel mappings cloned.
    unsafe {
        enter_user_mode(info.entry_point, USER_STACK_TOP, addr_space.cr3());
    }

    // We arrive here after return_to_kernel restores context
    core::mem::forget(addr_space);

    // Restore parent PID
    crate::syscall::table::set_current_pid(saved_pid);

    USER_EXIT_CODE.load(Ordering::SeqCst)
}

/// Load an ELF binary and jump to user mode (never returns).
pub fn exec_elf(elf_data: &[u8], hhdm_offset: u64) -> ! {
    let info = elf::parse(elf_data).expect("exec: invalid ELF");
    let addr_space = AddressSpace::new(hhdm_offset).expect("exec: address space");

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
                &elf_data[seg.offset as usize..(seg.offset + seg.filesz) as usize],
                seg.memsz,
                flags,
            )
            .expect("exec: load segment");
    }

    addr_space.map_user_stack().expect("exec: user stack");

    // SAFETY: Valid CR3 and IRETQ frame.
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) addr_space.cr3(), options(nostack));
        core::arch::asm!(
            "push {ss}",
            "push {rsp_user}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "iretq",
            ss = in(reg) gdt::USER_DS as u64,
            rsp_user = in(reg) USER_STACK_TOP,
            rflags = in(reg) 0x202u64,
            cs = in(reg) gdt::USER_CS as u64,
            rip = in(reg) info.entry_point,
            options(noreturn)
        );
    }
}
