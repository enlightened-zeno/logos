use crate::arch::x86_64::gdt;
use crate::memory::paging::PageFlags;
use crate::process::address_space::{AddressSpace, USER_STACK_TOP};
use crate::process::elf;

/// Load an ELF binary and jump to user mode.
///
/// This function never returns — it transitions to ring 3.
pub fn exec_elf(elf_data: &[u8], hhdm_offset: u64) -> ! {
    let info = elf::parse(elf_data).expect("exec: invalid ELF");

    // Create a new address space
    let addr_space = AddressSpace::new(hhdm_offset).expect("exec: address space");

    // Load each segment
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

    // Map user stack
    addr_space.map_user_stack().expect("exec: user stack");

    // Set up initial stack: push argc=0, argv=NULL, envp=NULL
    let stack_top = USER_STACK_TOP;

    // Switch to the new address space and jump to user mode
    jump_to_user(info.entry_point, stack_top, addr_space.cr3());
}

/// Transition to user mode via IRETQ.
///
/// Sets up a fake interrupt frame on the kernel stack and executes IRETQ
/// to switch to ring 3 with the given RIP, RSP, and CR3.
fn jump_to_user(entry: u64, user_rsp: u64, cr3: u64) -> ! {
    // Switch page tables
    // SAFETY: CR3 points to a valid PML4 with kernel mappings intact.
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack));
    }

    // Use IRETQ to jump to ring 3.
    // Stack frame for IRETQ: SS, RSP, RFLAGS, CS, RIP
    // SAFETY: We set up a valid IRETQ frame to enter user mode.
    unsafe {
        core::arch::asm!(
            "mov rsp, {kstack}",  // Use a clean kernel stack area
            "push {ss}",         // SS = user data segment
            "push {rsp_user}",   // RSP = user stack
            "push {rflags}",     // RFLAGS = IF=1 (interrupts enabled)
            "push {cs}",         // CS = user code segment
            "push {rip}",        // RIP = entry point
            "iretq",
            kstack = in(reg) user_rsp, // Temp: use user RSP area for IRETQ frame build
            ss = in(reg) gdt::USER_DS as u64,
            rsp_user = in(reg) user_rsp,
            rflags = in(reg) 0x202u64, // IF=1, reserved bit 1=1
            cs = in(reg) gdt::USER_CS as u64,
            rip = in(reg) entry,
            options(noreturn)
        );
    }
}
