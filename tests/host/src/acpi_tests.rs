/// ACPI/architecture logic tests.

#[test]
fn test_rsdp_signature() {
    let sig = b"RSD PTR ";
    assert_eq!(sig.len(), 8);
    assert_eq!(&sig[0..3], b"RSD");
}

#[test]
fn test_apic_base_msr() {
    let msr: u32 = 0x1B;
    assert_eq!(msr, 27);
}

#[test]
fn test_apic_base_address() {
    let default_base: u64 = 0xFEE0_0000;
    assert_eq!(default_base, 0xFEE00000);
}

#[test]
fn test_msr_star_encoding() {
    let kernel_cs: u64 = 0x08;
    let user_cs_base: u64 = 0x10;
    let star = (user_cs_base << 48) | (kernel_cs << 32);
    assert_eq!(star >> 32 & 0xFFFF, kernel_cs);
    assert_eq!(star >> 48 & 0xFFFF, user_cs_base);
}

#[test]
fn test_sfmask_bits() {
    let if_bit: u64 = 0x200;
    let tf_bit: u64 = 0x100;
    let df_bit: u64 = 0x400;
    let sfmask = if_bit | tf_bit | df_bit;
    assert_eq!(sfmask, 0x700);
}

#[test]
fn test_efer_sce() {
    let sce: u64 = 1 << 0;
    assert_eq!(sce, 1);
}
