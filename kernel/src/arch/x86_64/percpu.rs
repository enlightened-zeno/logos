/// Per-CPU data structure, accessed via GS segment base.
///
/// SWAPGS swaps between user GS (0) and kernel GS (this struct).
/// The syscall entry point uses gs:[0] for user RSP scratch and
/// gs:[8] for kernel RSP.
#[repr(C)]
pub struct PerCpuData {
    /// Scratch space for saving user RSP during syscall entry.
    pub user_rsp_scratch: u64,
    /// Kernel stack pointer (top of kernel stack for this CPU).
    pub kernel_rsp: u64,
}

/// MSR for kernel GS base (swapped in on SWAPGS).
const IA32_KERNEL_GS_BASE: u32 = 0xC000_0102;
/// MSR for current GS base.
const IA32_GS_BASE: u32 = 0xC000_0101;

static mut BSP_PERCPU: PerCpuData = PerCpuData {
    user_rsp_scratch: 0,
    kernel_rsp: 0,
};

/// Initialize per-CPU data for the BSP.
///
/// Sets KERNEL_GS_BASE MSR so that SWAPGS in the syscall entry
/// gives us access to our per-CPU data.
///
/// # Safety
/// Must be called once during boot after GDT/TSS setup.
pub unsafe fn init(kernel_stack_top: u64) {
    let percpu_ptr = core::ptr::addr_of_mut!(BSP_PERCPU);

    // SAFETY: Writing to the static per-CPU data during single-threaded boot.
    unsafe {
        (*percpu_ptr).kernel_rsp = kernel_stack_top;
    }

    let percpu_addr = percpu_ptr as u64;

    // Set KERNEL_GS_BASE so SWAPGS loads our per-CPU pointer
    // SAFETY: Setting MSR with valid per-CPU data address.
    unsafe {
        let lo = percpu_addr as u32;
        let hi = (percpu_addr >> 32) as u32;
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_KERNEL_GS_BASE,
            in("eax") lo,
            in("edx") hi,
            options(nomem, nostack)
        );
    }

    crate::serial_println!("Per-CPU: GS base set at {:#x}", percpu_addr);
}
