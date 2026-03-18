use crate::arch::x86_64::io;
use core::sync::atomic::{AtomicU64, Ordering};

/// APIC register offsets (from APIC base address).
const APIC_ID: u32 = 0x020;
const APIC_VERSION: u32 = 0x030;
const APIC_TPR: u32 = 0x080;
const APIC_EOI: u32 = 0x0B0;
const APIC_SVR: u32 = 0x0F0;
const APIC_ICR_LOW: u32 = 0x300;
const APIC_ICR_HIGH: u32 = 0x310;
const APIC_LVT_TIMER: u32 = 0x320;
const APIC_TIMER_INIT: u32 = 0x380;
const APIC_TIMER_CURRENT: u32 = 0x390;
const APIC_TIMER_DIV: u32 = 0x3E0;

/// APIC timer vector number.
pub const TIMER_VECTOR: u8 = 0x20;

/// APIC spurious interrupt vector.
const SPURIOUS_VECTOR: u8 = 0xFF;

/// MSR for APIC base address.
const IA32_APIC_BASE_MSR: u32 = 0x1B;

/// Virtual base address of the APIC MMIO region (set during init).
static APIC_BASE: AtomicU64 = AtomicU64::new(0);

/// Ticks per millisecond (calibrated against the PIT).
static TICKS_PER_MS: AtomicU64 = AtomicU64::new(0);

/// Global tick counter incremented by the timer ISR.
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Read from an APIC register.
fn read(offset: u32) -> u32 {
    let base = APIC_BASE.load(Ordering::Relaxed);
    // SAFETY: APIC MMIO region is mapped and base is valid after init.
    unsafe { core::ptr::read_volatile((base + offset as u64) as *const u32) }
}

/// Write to an APIC register.
fn write(offset: u32, value: u32) {
    let base = APIC_BASE.load(Ordering::Relaxed);
    // SAFETY: APIC MMIO region is mapped and base is valid after init.
    unsafe { core::ptr::write_volatile((base + offset as u64) as *mut u32, value) }
}

/// Read an MSR.
unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    // SAFETY: Caller must ensure the MSR exists.
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
    }
    (high as u64) << 32 | low as u64
}

/// Write an MSR.
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    // SAFETY: Caller must ensure the MSR exists and value is valid.
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack)
        );
    }
}

/// Initialize the Local APIC.
///
/// # Safety
/// Must be called once during boot after the GDT, IDT, and memory subsystem
/// are initialized. The HHDM must be active.
pub unsafe fn init(hhdm_offset: u64) {
    use crate::memory::addr::{PhysAddr, PhysFrame, VirtAddr};
    use crate::memory::paging::PageFlags;
    use crate::memory::vmm::Vmm;

    // Get the APIC base physical address from the MSR
    // SAFETY: IA32_APIC_BASE MSR exists on all x86_64 CPUs with APIC.
    let apic_base_msr = unsafe { rdmsr(IA32_APIC_BASE_MSR) };
    let apic_phys = apic_base_msr & 0xFFFF_FFFF_FFFF_F000;

    // The APIC MMIO region may not be covered by the HHDM (it maps RAM, not MMIO).
    // Explicitly map the APIC page as uncacheable, writable, no-execute.
    let apic_virt = apic_phys + hhdm_offset;
    let vmm = Vmm::get();
    let virt = VirtAddr::new_canonicalize(apic_virt);
    let frame = PhysFrame::containing_address(PhysAddr::new(apic_phys));
    // Map if not already mapped (Limine may or may not include MMIO in HHDM)
    let _ = vmm.map_page(
        virt,
        frame,
        PageFlags::WRITABLE
            | PageFlags::NO_EXECUTE
            | PageFlags::NO_CACHE
            | PageFlags::WRITE_THROUGH,
    );
    APIC_BASE.store(apic_virt, Ordering::Relaxed);

    // Enable the APIC via MSR (set bit 11)
    // SAFETY: Enabling the APIC through its base MSR.
    unsafe {
        wrmsr(IA32_APIC_BASE_MSR, apic_base_msr | (1 << 11));
    }

    // Set the spurious interrupt vector and enable the APIC (bit 8)
    write(APIC_SVR, SPURIOUS_VECTOR as u32 | 0x100);

    // Set task priority to 0 (accept all interrupts)
    write(APIC_TPR, 0);

    crate::serial_println!(
        "APIC: id={}, version={:#x}, phys={:#x}",
        read(APIC_ID) >> 24,
        read(APIC_VERSION),
        apic_phys
    );
}

/// Calibrate the APIC timer against the PIT.
///
/// Uses PIT channel 2 to measure how many APIC timer ticks occur in 10ms.
///
/// # Safety
/// Must be called after APIC init, with interrupts disabled.
pub unsafe fn calibrate_timer() {
    // Set APIC timer divider to 16
    write(APIC_TIMER_DIV, 0x03); // Divide by 16

    // Set up PIT channel 2 for a 10ms one-shot
    // PIT frequency is 1193182 Hz, so 10ms = 11932 ticks
    let pit_count: u16 = 11932;

    // Gate the PIT channel 2 speaker (bit 0 = gate, bit 1 = speaker)
    let port61 = io::inb(0x61);
    io::outb(0x61, (port61 & 0xFD) | 0x01); // Gate on, speaker off

    // PIT channel 2, mode 0 (interrupt on terminal count), binary
    io::outb(0x43, 0xB0);
    io::outb(0x42, pit_count as u8);
    io::outb(0x42, (pit_count >> 8) as u8);

    // Start APIC timer with max count
    write(APIC_TIMER_INIT, 0xFFFF_FFFF);

    // Wait for PIT to finish (poll bit 5 of port 0x61)
    loop {
        if io::inb(0x61) & 0x20 != 0 {
            break;
        }
    }

    // Read how many APIC ticks elapsed
    let elapsed = 0xFFFF_FFFFu32 - read(APIC_TIMER_CURRENT);
    write(APIC_TIMER_INIT, 0); // Stop timer

    // elapsed ticks in 10ms → ticks per ms
    let ticks_per_ms = elapsed as u64 / 10;
    TICKS_PER_MS.store(ticks_per_ms, Ordering::Relaxed);

    crate::serial_println!("APIC timer: {} ticks/ms (div=16)", ticks_per_ms);
}

/// Start the APIC timer in periodic mode at ~1000 Hz.
pub fn start_periodic() {
    let ticks_per_ms = TICKS_PER_MS.load(Ordering::Relaxed);
    if ticks_per_ms == 0 {
        panic!("APIC timer not calibrated");
    }

    // 1 ms period = 1000 Hz
    write(APIC_TIMER_DIV, 0x03); // Divide by 16
                                 // LVT Timer: periodic mode (bit 17), vector TIMER_VECTOR
    write(APIC_LVT_TIMER, (1 << 17) | TIMER_VECTOR as u32);
    write(APIC_TIMER_INIT, ticks_per_ms as u32);
}

/// Send End-Of-Interrupt to the APIC.
#[inline]
pub fn eoi() {
    write(APIC_EOI, 0);
}

/// Increment the global tick counter. Called from the timer ISR.
pub fn tick() {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Get the current tick count.
pub fn ticks() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

/// Get the APIC ID of the current CPU.
pub fn id() -> u32 {
    read(APIC_ID) >> 24
}
