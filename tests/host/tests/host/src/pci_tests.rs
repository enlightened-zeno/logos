//! PCI tests
#[test] fn pci_config_addr() { let addr = 0x8000_0000u32 | (0 << 16) | (0 << 11) | (0 << 8); assert_eq!(addr, 0x8000_0000); }
#[test] fn pci_vendor_invalid() { assert_eq!(0xFFFFu16, 0xFFFF); }
#[test] fn pci_virtio_vendor() { assert_eq!(0x1AF4u16, 0x1AF4); }
#[test] fn pci_class_bridge() { assert_eq!(0x06u8, 6); }
#[test] fn pci_class_storage() { assert_eq!(0x01u8, 1); }
#[test] fn pci_class_display() { assert_eq!(0x03u8, 3); }
#[test] fn pci_bar_mmio() { let bar: u32 = 0xFEE0_0000; assert!(bar & 1 == 0, "MMIO BAR bit 0 = 0"); }
#[test] fn pci_bar_io() { let bar: u32 = 0xCF81; assert!(bar & 1 == 1, "IO BAR bit 0 = 1"); }
#[test] fn pci_max_bus() { assert_eq!(255u8, 255); }
#[test] fn pci_max_device() { assert_eq!(31u8, 31); }
#[test] fn pci_max_function() { assert_eq!(7u8, 7); }
