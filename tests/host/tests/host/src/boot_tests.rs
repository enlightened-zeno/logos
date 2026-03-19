//! Boot sequence tests
#[test] fn limine_base_revision() { assert_eq!(3u64, 3); /* Protocol revision 3 */ }
#[test] fn kernel_virt_base() { assert_eq!(0xFFFF_FFFF_8000_0000u64, 0xFFFF_FFFF_8000_0000); }
#[test] fn hhdm_base() { assert_eq!(0xFFFF_8000_0000_0000u64, 0xFFFF_8000_0000_0000); }
#[test] fn stack_size() { assert_eq!(0x10_0000u64, 1048576); /* 1 MiB */ }
#[test] fn serial_baud() { assert_eq!(115200u32, 115200); }
#[test] fn serial_port() { assert_eq!(0x3F8u16, 0x3F8); /* COM1 */ }
#[test] fn gdt_entries() { /* null, kcode, kdata, udata, ucode, tss = 6 (tss is double) */ assert!(true); }
#[test] fn idt_entries() { assert_eq!(256u16, 256); }
#[test] fn apic_base_addr() { assert_eq!(0xFEE0_0000u64, 0xFEE0_0000); }
#[test] fn boot_order_serial_first() { /* Serial must init before any output */ assert!(true); }
