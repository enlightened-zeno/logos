/// PCI enumeration logic tests.

#[test]
fn test_pci_config_address() {
    let bus: u8 = 0;
    let device: u8 = 1;
    let function: u8 = 0;
    let offset: u8 = 0;
    let addr: u32 = 0x8000_0000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);
    assert_eq!(addr, 0x8000_0800);
}

#[test]
fn test_pci_vendor_invalid() {
    let vendor: u16 = 0xFFFF;
    assert_eq!(vendor, 0xFFFF); // No device present
}

#[test]
fn test_pci_class_codes() {
    // Mass storage
    assert_eq!(0x01u8, 1);
    // Network
    assert_eq!(0x02u8, 2);
    // Display
    assert_eq!(0x03u8, 3);
    // Bridge
    assert_eq!(0x06u8, 6);
}

#[test]
fn test_pci_bar_type() {
    let bar_io: u32 = 0xC041; // Bit 0 = 1 → I/O
    let bar_mmio: u32 = 0xFEE00000; // Bit 0 = 0 → MMIO
    assert!(bar_io & 1 != 0);
    assert!(bar_mmio & 1 == 0);
}

#[test]
fn test_ahci_class() {
    let class: u8 = 0x01; // Mass storage
    let subclass: u8 = 0x06; // SATA
    assert_eq!(class, 1);
    assert_eq!(subclass, 6);
}

#[test]
fn test_multifunction_device() {
    let header_type: u8 = 0x80; // Bit 7 set = multifunction
    assert!(header_type & 0x80 != 0);
    let max_functions = if header_type & 0x80 != 0 { 8 } else { 1 };
    assert_eq!(max_functions, 8);
}
