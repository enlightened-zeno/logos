use core::mem::size_of;

/// GDT segment selectors per spec: null, kernel code, kernel data, user data, user code, TSS.
pub const KERNEL_CS: u16 = 0x08;
pub const KERNEL_DS: u16 = 0x10;
pub const USER_DS: u16 = 0x18 | 3; // RPL 3
pub const USER_CS: u16 = 0x20 | 3; // RPL 3
const TSS_SELECTOR: u16 = 0x28;

/// Number of IST entries available (1-indexed, 1..=7).
pub const IST_DOUBLE_FAULT: u16 = 1;
pub const IST_NMI: u16 = 2;
pub const IST_MCE: u16 = 3;

const IST_STACK_SIZE: usize = 4096 * 4; // 16 KiB per IST stack

/// Task State Segment for x86_64.
#[repr(C, packed)]
pub struct Tss {
    _reserved0: u32,
    /// Privilege stack table (RSP for ring 0, 1, 2).
    pub rsp: [u64; 3],
    _reserved1: u64,
    /// Interrupt Stack Table entries (IST1..IST7).
    pub ist: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    /// I/O map base address (offset from TSS base).
    pub iomap_base: u16,
}

impl Tss {
    const fn new() -> Self {
        Self {
            _reserved0: 0,
            rsp: [0; 3],
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            iomap_base: size_of::<Tss>() as u16,
        }
    }
}

/// A single GDT entry (8 bytes for normal segments).
#[derive(Clone, Copy)]
#[repr(transparent)]
struct GdtEntry(u64);

impl GdtEntry {
    const fn null() -> Self {
        Self(0)
    }

    /// Kernel code segment: 64-bit, present, DPL 0, execute/read.
    const fn kernel_code() -> Self {
        // L=1 (long mode), P=1, S=1 (code/data), Type=0xA (exec/read)
        Self(0x00AF_9A00_0000_FFFF)
    }

    /// Kernel data segment: present, DPL 0, read/write.
    const fn kernel_data() -> Self {
        // P=1, S=1, Type=0x2 (read/write)
        Self(0x00CF_9200_0000_FFFF)
    }

    /// User data segment: present, DPL 3, read/write.
    const fn user_data() -> Self {
        // P=1, S=1, DPL=3, Type=0x2
        Self(0x00CF_F200_0000_FFFF)
    }

    /// User code segment: 64-bit, present, DPL 3, execute/read.
    const fn user_code() -> Self {
        // L=1, P=1, S=1, DPL=3, Type=0xA
        Self(0x00AF_FA00_0000_FFFF)
    }
}

/// TSS descriptor is 16 bytes (two GDT slots).
#[derive(Clone, Copy)]
#[repr(C)]
struct TssDescriptor {
    low: u64,
    high: u64,
}

impl TssDescriptor {
    fn new(tss_addr: u64, tss_size: u16) -> Self {
        let limit = tss_size - 1;
        let base_low = tss_addr & 0xFFFF;
        let base_mid = (tss_addr >> 16) & 0xFF;
        let base_mid2 = (tss_addr >> 24) & 0xFF;
        let base_high = tss_addr >> 32;

        let low = (limit as u64)
            | (base_low << 16)
            | (base_mid << 32)
            | (0x89u64 << 40) // Present, 64-bit TSS available
            | (base_mid2 << 56);

        let high = base_high;

        Self { low, high }
    }
}

/// GDT with 5 normal entries + 1 TSS descriptor (2 slots) = 7 slots.
#[repr(C, align(16))]
struct Gdt {
    entries: [u64; 7],
}

/// GDTR pointer structure.
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

// Per-CPU GDT, TSS, and IST stacks. For now, single CPU (BSP only).
static mut BSP_TSS: Tss = Tss::new();
static mut BSP_GDT: Gdt = Gdt { entries: [0; 7] };
static mut IST_STACKS: [[u8; IST_STACK_SIZE]; 3] = [[0; IST_STACK_SIZE]; 3];
static mut KERNEL_STACK: [u8; IST_STACK_SIZE] = [0; IST_STACK_SIZE];

/// Initialize the GDT and TSS for the bootstrap processor.
///
/// # Safety
/// Must be called once during single-threaded boot.
pub unsafe fn init() {
    use core::ptr::addr_of_mut;

    // SAFETY: Single-threaded boot, these statics are only written here.
    // Using addr_of_mut! to avoid creating references to static mut.
    let tss = addr_of_mut!(BSP_TSS);
    let gdt = addr_of_mut!(BSP_GDT);

    // SAFETY: Single-threaded boot. TSS and GDT pointers from addr_of_mut
    // are valid, and we write before loading them into CPU registers.
    unsafe {
        (*tss).ist[0] = ist_stack_top(0); // IST1: double fault
        (*tss).ist[1] = ist_stack_top(1); // IST2: NMI
        (*tss).ist[2] = ist_stack_top(2); // IST3: MCE

        // Kernel RSP0 (used on ring 3 → ring 0 transitions)
        (*tss).rsp[0] = addr_of_mut!(KERNEL_STACK) as u64 + IST_STACK_SIZE as u64;
    }

    // Build the GDT
    let tss_addr = tss as u64;
    let tss_desc = TssDescriptor::new(tss_addr, size_of::<Tss>() as u16);

    // SAFETY: Single-threaded boot.
    unsafe {
        (*gdt).entries[0] = GdtEntry::null().0;
        (*gdt).entries[1] = GdtEntry::kernel_code().0;
        (*gdt).entries[2] = GdtEntry::kernel_data().0;
        (*gdt).entries[3] = GdtEntry::user_data().0;
        (*gdt).entries[4] = GdtEntry::user_code().0;
        (*gdt).entries[5] = tss_desc.low;
        (*gdt).entries[6] = tss_desc.high;
    }

    // Load the GDT
    let gdt_ptr = GdtPointer {
        limit: (size_of::<Gdt>() - 1) as u16,
        base: gdt as u64,
    };

    // SAFETY: We're loading a valid GDT with correct segment descriptors.
    unsafe {
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdt_ptr,
            options(nostack)
        );
    }

    // Reload segment registers
    reload_segments();

    // Load the TSS
    // SAFETY: TSS descriptor is valid at index 5 in the GDT.
    unsafe {
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) TSS_SELECTOR,
            options(nostack, nomem)
        );
    }
}

/// Reload CS (via far return) and data segment registers.
///
/// # Safety
/// GDT must be loaded with valid kernel code/data segments.
unsafe fn reload_segments() {
    // SAFETY: Performing a far return to reload CS, then loading data segments.
    unsafe {
        core::arch::asm!(
            // Push kernel CS and the address of the next instruction for retfq
            "push {cs}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            // Reload data segments
            "mov ds, {ds:e}",
            "mov es, {ds:e}",
            "mov ss, {ds:e}",
            // Zero FS and GS for now (will be set up for per-CPU data later)
            "xor {zero:e}, {zero:e}",
            "mov fs, {zero:e}",
            "mov gs, {zero:e}",
            cs = in(reg) KERNEL_CS as u64,
            ds = in(reg) KERNEL_DS as u64,
            tmp = lateout(reg) _,
            zero = lateout(reg) _,
            options(preserves_flags),
        );
    }
}

fn ist_stack_top(index: usize) -> u64 {
    // SAFETY: IST_STACKS is a static array, using addr_of_mut to avoid reference.
    unsafe { core::ptr::addr_of_mut!(IST_STACKS[index]) as u64 + IST_STACK_SIZE as u64 }
}
