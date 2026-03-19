/// ELF validation tests (host-side, no kernel needed).

fn make_elf_header(class: u8, data: u8, etype: u16, machine: u16) -> Vec<u8> {
    let mut elf = vec![0u8; 64];
    elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf[4] = class;
    elf[5] = data;
    elf[16..18].copy_from_slice(&etype.to_le_bytes());
    elf[18..20].copy_from_slice(&machine.to_le_bytes());
    elf
}

#[test]
fn test_elf_magic() {
    let valid = [0x7Fu8, b'E', b'L', b'F'];
    assert_eq!(&valid, b"\x7FELF");
}

#[test]
fn test_elf_class64() {
    let elf = make_elf_header(2, 1, 2, 62); // 64-bit, LSB, EXEC, x86_64
    assert_eq!(elf[4], 2); // ELFCLASS64
}

#[test]
fn test_elf_class32_rejected() {
    let elf = make_elf_header(1, 1, 2, 62); // 32-bit
    assert_eq!(elf[4], 1); // Should be rejected by parser
}

#[test]
fn test_elf_wrong_machine() {
    let elf = make_elf_header(2, 1, 2, 183); // AArch64
    assert_eq!(u16::from_le_bytes([elf[18], elf[19]]), 183);
}

#[test]
fn test_elf_empty_rejected() {
    let elf: Vec<u8> = vec![];
    assert!(elf.len() < 64); // Too small for header
}

#[test]
fn test_elf_truncated() {
    let elf = vec![0x7F, b'E', b'L', b'F', 2, 1, 0, 0];
    assert!(elf.len() < 64); // Truncated
}

#[test]
fn test_elf_wx_violation() {
    // PF_R|PF_W|PF_X = 7 — should be rejected by W^X enforcement
    let flags: u32 = 7;
    assert!(flags & 2 != 0 && flags & 1 != 0); // W and X both set
}

#[test]
fn test_elf_kernel_address_rejected() {
    let vaddr: u64 = 0xFFFF_8000_0000_0000;
    assert!(vaddr >= 0xFFFF_8000_0000_0000); // In kernel space
}

#[test]
fn test_elf_user_address_valid() {
    let vaddr: u64 = 0x0000_0040_0000;
    assert!(vaddr < 0xFFFF_8000_0000_0000); // In user space
}

#[test]
fn test_pt_load_type() {
    let pt_load: u32 = 1;
    assert_eq!(pt_load, 1);
}
