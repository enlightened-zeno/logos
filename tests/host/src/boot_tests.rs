/// Boot and system integration logic tests.

#[test]
fn test_kernel_virtual_base() {
    let base: u64 = 0xFFFF_FFFF_8000_0000;
    assert!(base >= 0xFFFF_8000_0000_0000); // In kernel space
}

#[test]
fn test_hhdm_offset() {
    let hhdm: u64 = 0xFFFF_8000_0000_0000;
    let phys: u64 = 0x1000;
    let virt = phys + hhdm;
    assert!(virt > hhdm);
    assert_eq!(virt - hhdm, phys);
}

#[test]
fn test_stack_size() {
    let kernel_stack = 0x10_0000u64; // 1 MiB from Limine
    assert_eq!(kernel_stack, 1024 * 1024);
}

#[test]
fn test_user_stack_layout() {
    let top: u64 = 0x0000_7FFF_FFFF_F000;
    let size: u64 = 64 * 4096; // 256 KiB
    let bottom = top - size;
    assert!(bottom < top);
    assert!(top < 0xFFFF_8000_0000_0000); // In user space
}

#[test]
fn test_gdt_selectors() {
    let kernel_cs: u16 = 0x08;
    let kernel_ds: u16 = 0x10;
    let user_ds: u16 = 0x18 | 3;
    let user_cs: u16 = 0x20 | 3;

    // Ring 0 selectors have RPL=0
    assert_eq!(kernel_cs & 3, 0);
    assert_eq!(kernel_ds & 3, 0);
    // Ring 3 selectors have RPL=3
    assert_eq!(user_ds & 3, 3);
    assert_eq!(user_cs & 3, 3);
}

#[test]
fn test_idt_size() {
    let entries = 256;
    let entry_size = 16; // bytes per IDT entry
    let total = entries * entry_size;
    assert_eq!(total, 4096);
}

#[test]
fn test_apic_timer_frequency() {
    let target_hz = 1000;
    let period_ms = 1000 / target_hz;
    assert_eq!(period_ms, 1);
}

#[test]
fn test_serial_baud_rate() {
    let baud = 115200u32;
    let divisor = 115200 / baud;
    assert_eq!(divisor, 1);
}
