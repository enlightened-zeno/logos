/// I/O port and MMIO logic tests.

#[test]
fn test_io_port_ranges() {
    let com1: u16 = 0x3F8;
    let pci_config: u16 = 0xCF8;
    let pci_data: u16 = 0xCFC;
    let pic1_cmd: u16 = 0x20;
    let pic2_cmd: u16 = 0xA0;
    let kbd_data: u16 = 0x60;
    let kbd_status: u16 = 0x64;
    let qemu_exit: u16 = 0xF4;
    let acpi_shutdown: u16 = 0x604;

    assert!(com1 < 0x1000);
    assert_ne!(pci_config, pci_data);
    assert_ne!(pic1_cmd, pic2_cmd);
    assert_ne!(kbd_data, kbd_status);
    let _ = (qemu_exit, acpi_shutdown);
}

#[test]
fn test_mmio_volatile() {
    // MMIO reads/writes must be volatile
    let mut val: u32 = 0;
    unsafe {
        // SAFETY: Just a test with a local variable
        core::ptr::write_volatile(&mut val, 42);
        assert_eq!(core::ptr::read_volatile(&val), 42);
    }
}

#[test]
fn test_pit_frequency() {
    let pit_freq: u32 = 1193182;
    let target_10ms: u16 = (pit_freq / 100) as u16;
    assert_eq!(target_10ms, 11931);
}

#[test]
fn test_apic_register_offsets() {
    let id: u32 = 0x020;
    let eoi: u32 = 0x0B0;
    let svr: u32 = 0x0F0;
    let timer_lvt: u32 = 0x320;
    let timer_init: u32 = 0x380;
    let timer_div: u32 = 0x3E0;
    assert!(id < eoi);
    assert!(eoi < svr);
    assert!(timer_lvt < timer_init);
    assert!(timer_init < timer_div);
}

#[test]
fn test_apic_timer_divider() {
    // Divider value 0x03 = divide by 16
    let div: u32 = 0x03;
    assert_eq!(div, 3);
}
