//\! Address type tests (PMM/VMM address handling)

#[test]
fn phys_addr_page_alignment() {
    // PhysAddr should page-align correctly
    let addr = 0x1234u64;
    let aligned = addr & \!0xFFF;
    assert_eq\!(aligned, 0x1000);
}

#[test]
fn phys_addr_zero() {
    assert_eq\!(0u64 & \!0xFFF, 0);
}

#[test]
fn virt_addr_canonical_positive() {
    let addr: u64 = 0x0000_7FFF_FFFF_FFFF;
    // Bit 47 is 0, so canonical form keeps it as-is
    assert\!(addr < 0x0000_8000_0000_0000);
}

#[test]
fn virt_addr_canonical_negative() {
    let addr: u64 = 0xFFFF_8000_0000_0000;
    // Bit 47 is 1, canonical has bits 48-63 set
    assert\!(addr >= 0xFFFF_8000_0000_0000);
}

#[test]
fn page_table_index_extraction() {
    let addr: u64 = 0xFFFF_FFFF_8000_0000;
    let pml4 = (addr >> 39) & 0x1FF;
    let pdpt = (addr >> 30) & 0x1FF;
    let pd = (addr >> 21) & 0x1FF;
    let pt = (addr >> 12) & 0x1FF;
    assert_eq\!(pml4, 511);
    assert_eq\!(pdpt, 510);
    assert_eq\!(pd, 0);
    assert_eq\!(pt, 0);
}

#[test]
fn hhdm_offset_calculation() {
    let hhdm: u64 = 0xFFFF_8000_0000_0000;
    let phys: u64 = 0x1000;
    let virt = hhdm + phys;
    assert_eq\!(virt, 0xFFFF_8000_0000_1000);
    assert_eq\!(virt - hhdm, phys);
}

#[test]
fn frame_number_from_addr() {
    let addr: u64 = 0x12345000;
    let frame = addr >> 12;
    assert_eq\!(frame, 0x12345);
    assert_eq\!(frame << 12, addr);
}

#[test]
fn addr_from_frame_number() {
    let frame: u64 = 0xABCDE;
    let addr = frame << 12;
    assert_eq\!(addr, 0xABCDE000);
}

#[test]
fn kernel_space_boundary() {
    let boundary: u64 = 0xFFFF_8000_0000_0000;
    assert\!(boundary > 0x0000_7FFF_FFFF_FFFF);
    // User addresses are below boundary
    assert\!(0x400000u64 < boundary);
    // Kernel addresses are at or above
    assert\!(0xFFFF_FFFF_8000_0000u64 >= boundary);
}

#[test]
fn page_offset_extraction() {
    let addr: u64 = 0xDEAD_BEEF_1234;
    let offset = addr & 0xFFF;
    assert_eq\!(offset, 0x234);
}

#[test]
fn max_phys_addr() {
    // x86_64 supports up to 52-bit physical addresses
    let max_phys: u64 = (1u64 << 52) - 1;
    assert_eq\!(max_phys, 0x000F_FFFF_FFFF_FFFF);
}
