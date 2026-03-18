#![allow(dead_code)]

/// Halt the CPU until the next interrupt.
#[inline(always)]
pub fn hlt() {
    // SAFETY: HLT is safe to execute — it simply waits for the next interrupt.
    // The CPU will resume execution when an interrupt arrives.
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack));
    }
}

/// Disable interrupts on the current CPU.
#[inline(always)]
pub fn cli() {
    // SAFETY: CLI disables interrupts. This is safe as long as we re-enable
    // them in a timely manner to avoid missed hardware events.
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Enable interrupts on the current CPU.
#[inline(always)]
pub fn sti() {
    // SAFETY: STI enables interrupts. This is always safe to call.
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Read the RFLAGS register.
#[inline(always)]
pub fn read_rflags() -> u64 {
    let rflags: u64;
    // SAFETY: PUSHFQ/POP reads the current RFLAGS register value.
    // No side effects beyond reading CPU state.
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem),
        );
    }
    rflags
}

/// Check if interrupts are currently enabled.
#[inline(always)]
pub fn interrupts_enabled() -> bool {
    read_rflags() & (1 << 9) != 0
}

/// Execute CPUID instruction with the given leaf and sub-leaf.
#[inline]
pub fn cpuid(leaf: u32, sub_leaf: u32) -> CpuidResult {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    // SAFETY: CPUID is always safe to call. It reads CPU identification
    // information without any side effects on system state. We save/restore
    // RBX because LLVM reserves it.
    unsafe {
        core::arch::asm!(
            "mov {tmp:r}, rbx",
            "cpuid",
            "xchg {tmp:r}, rbx",
            tmp = lateout(reg) ebx,
            inlateout("eax") leaf => eax,
            inlateout("ecx") sub_leaf => ecx,
            lateout("edx") edx,
            options(nomem, nostack),
        );
    }
    CpuidResult { eax, ebx, ecx, edx }
}

/// Result of a CPUID instruction.
#[derive(Debug, Clone, Copy)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Detected CPU feature set.
#[derive(Debug)]
pub struct CpuFeatures {
    pub has_apic: bool,
    pub has_tsc: bool,
    pub has_fxsave: bool,
    pub has_sse2: bool,
    pub has_xsave: bool,
    pub has_rdrand: bool,
    pub has_rdseed: bool,
    pub has_nx: bool,
    pub has_syscall: bool,
    pub has_1gib_pages: bool,
    pub xsave_area_size: u32,
    pub vendor: [u8; 12],
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
}

impl CpuFeatures {
    /// Detect CPU features via CPUID.
    pub fn detect() -> Self {
        // Leaf 0: Vendor string
        let leaf0 = cpuid(0, 0);
        let mut vendor = [0u8; 12];
        vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
        vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());

        // Leaf 1: Feature bits + family/model/stepping
        let leaf1 = cpuid(1, 0);
        let stepping = (leaf1.eax & 0xF) as u8;
        let mut model = ((leaf1.eax >> 4) & 0xF) as u8;
        let mut family = ((leaf1.eax >> 8) & 0xF) as u8;
        if family == 0xF {
            family += ((leaf1.eax >> 20) & 0xFF) as u8;
        }
        if family == 0x6 || family == 0xF {
            model += (((leaf1.eax >> 16) & 0xF) as u8) << 4;
        }

        let has_apic = leaf1.edx & (1 << 9) != 0;
        let has_tsc = leaf1.edx & (1 << 4) != 0;
        let has_fxsave = leaf1.edx & (1 << 24) != 0;
        let has_sse2 = leaf1.edx & (1 << 26) != 0;
        let has_xsave = leaf1.ecx & (1 << 26) != 0;
        let has_rdrand = leaf1.ecx & (1 << 30) != 0;

        // Leaf 7: Extended features (sub-leaf 0)
        let leaf7 = cpuid(7, 0);
        let has_rdseed = leaf7.ebx & (1 << 18) != 0;

        // Leaf 0x80000001: Extended features
        let leaf_ext1 = cpuid(0x8000_0001, 0);
        let has_nx = leaf_ext1.edx & (1 << 20) != 0;
        let has_syscall = leaf_ext1.edx & (1 << 11) != 0;
        let has_1gib_pages = leaf_ext1.edx & (1 << 26) != 0;

        // XSAVE area size (leaf 0xD, sub-leaf 0)
        let xsave_area_size = if has_xsave {
            cpuid(0xD, 0).ecx
        } else {
            512 // FXSAVE legacy area size
        };

        CpuFeatures {
            has_apic,
            has_tsc,
            has_fxsave,
            has_sse2,
            has_xsave,
            has_rdrand,
            has_rdseed,
            has_nx,
            has_syscall,
            has_1gib_pages,
            xsave_area_size,
            vendor,
            family,
            model,
            stepping,
        }
    }

    /// Validate that all required CPU features are present.
    /// Returns an error message for the first missing feature, or None if all are present.
    pub fn validate(&self) -> Option<&'static str> {
        if !self.has_apic {
            return Some("APIC");
        }
        if !self.has_tsc {
            return Some("TSC");
        }
        if !self.has_fxsave {
            return Some("FXSAVE/FXRSTOR");
        }
        if !self.has_sse2 {
            return Some("SSE2");
        }
        if !self.has_nx {
            return Some("NX bit");
        }
        if !self.has_syscall {
            return Some("SYSCALL/SYSRET");
        }
        None
    }
}
