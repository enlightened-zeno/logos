use crate::arch::x86_64::{apic, gdt, idt};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use limine::response::MpResponse;

/// Number of CPUs that have completed initialization.
static CPUS_ONLINE: AtomicU32 = AtomicU32::new(1); // BSP is already online
static SMP_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Maximum supported CPUs.
pub const MAX_CPUS: usize = 64;

/// Initialize SMP using the Limine MP response.
///
/// # Safety
/// Must be called once during boot after GDT, IDT, and APIC are initialized.
pub unsafe fn init(response: &MpResponse, hhdm_offset: u64) {
    let cpus = response.cpus();
    let bsp_id = response.bsp_lapic_id();

    crate::serial_println!("SMP: {} CPUs detected, BSP LAPIC ID={}", cpus.len(), bsp_id);

    let mut ap_count = 0u32;

    for cpu in cpus {
        if cpu.lapic_id == bsp_id {
            continue;
        }

        ap_count += 1;
        if ap_count as usize >= MAX_CPUS {
            crate::serial_println!("SMP: too many CPUs, max={}", MAX_CPUS);
            break;
        }

        // Store HHDM offset in the CPU's extra field for the AP to read
        cpu.extra.store(hhdm_offset, Ordering::Release);

        // Boot this AP — writing to goto_address causes it to jump there
        cpu.goto_address.write(ap_entry);
    }

    // Wait for all APs to come online (with timeout)
    let expected = ap_count + 1; // +1 for BSP
    let mut spins = 0u64;
    while CPUS_ONLINE.load(Ordering::Acquire) < expected {
        core::hint::spin_loop();
        spins += 1;
        if spins > 100_000_000 {
            crate::serial_println!(
                "SMP: timeout waiting for APs ({}/{} online)",
                CPUS_ONLINE.load(Ordering::Relaxed),
                expected
            );
            break;
        }
    }

    let online = CPUS_ONLINE.load(Ordering::Relaxed);
    SMP_INITIALIZED.store(true, Ordering::Release);
    crate::serial_println!("SMP: {}/{} CPUs online", online, expected);
}

/// Entry point for Application Processors.
///
/// Called by Limine after AP startup. Each AP has its own stack from Limine.
unsafe extern "C" fn ap_entry(info: &limine::mp::Cpu) -> ! {
    // Each AP needs its own GDT and IDT
    // SAFETY: Each AP initializes its own CPU-local structures.
    unsafe {
        gdt::init();
        idt::init();
    }

    // Initialize this AP's Local APIC
    let hhdm = info.extra.load(Ordering::Acquire);
    // SAFETY: HHDM offset was stored by the BSP before booting this AP.
    unsafe {
        apic::init(hhdm);
    }

    let id = CPUS_ONLINE.fetch_add(1, Ordering::Release);
    crate::serial_println!("  AP {} online (LAPIC ID={})", id, info.lapic_id);

    // AP idle loop — halts until interrupted
    loop {
        crate::arch::x86_64::cpu::hlt();
    }
}

/// Get the number of CPUs currently online.
pub fn cpus_online() -> u32 {
    CPUS_ONLINE.load(Ordering::Relaxed)
}

/// Check if SMP has been initialized.
pub fn is_initialized() -> bool {
    SMP_INITIALIZED.load(Ordering::Relaxed)
}
