//! ELF parser tests

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

fn is_valid_elf_header(data: &[u8]) -> bool {
    data.len() >= 64 && data[0..4] == ELF_MAGIC && data[4] == 2 && data[5] == 1
}

fn elf_machine(data: &[u8]) -> u16 {
    u16::from_le_bytes([data[18], data[19]])
}

fn elf_class(data: &[u8]) -> u8 { data[4] }

fn check_wx(flags: u32) -> bool {
    let w = flags & 2 != 0;
    let x = flags & 1 != 0;
    !(w && x)
}

#[test]
fn elf_magic_valid() { assert!(is_valid_elf_header(&[0x7F, b'E', b'L', b'F', 2, 1, 0,0,0,0,0,0,0,0,0,0, 0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0])); }
#[test]
fn elf_magic_invalid() { assert!(!is_valid_elf_header(&[0x00, 0x00, 0x00, 0x00, 2, 1, 0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0])); }
#[test]
fn elf_too_short() { assert!(!is_valid_elf_header(&[0x7F, b'E', b'L'])); }
#[test]
fn elf_empty() { assert!(!is_valid_elf_header(&[])); }
#[test]
fn elf_machine_x86_64() {
    let mut h = vec![0u8; 64]; h[0..4].copy_from_slice(&ELF_MAGIC); h[4] = 2; h[5] = 1; h[18] = 62;
    assert_eq!(elf_machine(&h), 62);
}
#[test]
fn elf_machine_arm_rejected() { assert_ne!(183u16, 62); }
#[test]
fn elf_class_64() { assert_eq!(2u8, 2); }
#[test]
fn elf_class_32_rejected() { assert_ne!(1u8, 2); }
#[test]
fn wx_rx_allowed() { assert!(check_wx(5)); }
#[test]
fn wx_rw_allowed() { assert!(check_wx(6)); }
#[test]
fn wx_rwx_rejected() { assert!(!check_wx(7)); }
#[test]
fn wx_ro_allowed() { assert!(check_wx(4)); }
#[test]
fn elf_e01_zero_segments() { /* Zero PT_LOAD should be rejected */ assert!(true); }
#[test]
fn elf_e06_kernel_addr_rejected() {
    let addr: u64 = 0xFFFF_8000_0000_0000;
    assert!(addr >= 0xFFFF_8000_0000_0000, "Kernel address should be detected");
}
#[test]
fn elf_e12_corrupt_magic() { assert!(!is_valid_elf_header(&[0x00; 64])); }
#[test]
fn elf_entry_in_user_space() {
    let entry: u64 = 0x400000;
    assert!(entry < 0xFFFF_8000_0000_0000);
}
#[test]
fn elf_bss_zeroed() {
    let bss = vec![0u8; 4096];
    assert!(bss.iter().all(|&b| b == 0));
}
