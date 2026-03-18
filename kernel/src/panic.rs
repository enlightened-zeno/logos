use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

/// Flag to detect nested panics (panic inside panic handler).
static PANICKING: AtomicBool = AtomicBool::new(false);

/// Kernel panic handler. Outputs diagnostic information to the serial port
/// and halts the system.
pub fn panic_handler(info: &PanicInfo) -> ! {
    // Disable interrupts to prevent further corruption
    crate::arch::x86_64::cpu::cli();

    // Detect nested panic
    if PANICKING.swap(true, Ordering::SeqCst) {
        // We're already panicking — double panic. Minimal output and halt.
        crate::serial_println!("\n!!! DOUBLE PANIC !!!");
        loop {
            crate::arch::x86_64::cpu::hlt();
        }
    }

    crate::serial_println!("\n===== KERNEL PANIC =====");

    if let Some(location) = info.location() {
        crate::serial_println!(
            "Location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    if let Some(message) = info.message().as_str() {
        crate::serial_println!("Message: {}", message);
    } else {
        crate::serial_println!("Message: {}", info.message());
    }

    crate::serial_println!("========================\n");

    // Halt all CPUs
    loop {
        crate::arch::x86_64::cpu::hlt();
    }
}
