use crate::arch::x86_64::io::outl;

/// Exit codes written to the QEMU ISA debug exit device.
/// QEMU translates: exit_code = (written_value << 1) | 1
/// So 0x10 => 33 (success), 0x11 => 35 (failure).
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failure = 0x11,
}

/// Exit QEMU via the ISA debug exit device at port 0xF4.
pub fn exit_qemu(code: QemuExitCode) {
    outl(0xF4, code as u32);
}

/// Trait for test cases that can be run by the test runner.
pub trait Testable {
    fn run(&self) -> Result<(), &'static str>;
    fn name(&self) -> &str;
}

/// Run all registered test cases, print results to serial, and exit QEMU.
pub fn test_runner(tests: &[&dyn Testable]) {
    crate::serial_println!("\n=== LogOS Kernel Test Suite ===");
    crate::serial_println!("Running {} tests\n", tests.len());

    let mut passed = 0u32;
    let mut failed = 0u32;

    for test in tests {
        crate::serial_print!("  {} ... ", test.name());
        match test.run() {
            Ok(()) => {
                crate::serial_println!("[PASS]");
                passed += 1;
            }
            Err(msg) => {
                crate::serial_println!("[FAIL]");
                crate::serial_println!("    {}", msg);
                failed += 1;
            }
        }
    }

    crate::serial_println!("\n─────────────────────────────────────");
    crate::serial_println!("Results: {} passed, {} failed", passed, failed);
    crate::serial_println!("─────────────────────────────────────\n");

    if failed == 0 {
        exit_qemu(QemuExitCode::Success);
    } else {
        exit_qemu(QemuExitCode::Failure);
    }
}
