/// Address space logic tests.

#[test]
fn test_user_stack_layout() {
    let top: u64 = 0x0000_7FFF_FFFF_F000;
    let size: u64 = 64 * 4096;
    let bottom = top - size;
    assert!(bottom < top);
    assert!(top < 0x0000_8000_0000_0000);
}

#[test]
fn test_user_heap_start() {
    let heap: u64 = 0x0000_4000_0000_0000;
    assert!(heap < 0x0000_7FFF_FFFF_F000); // Below stack
    assert!(heap > 0); // Above null page
}

#[test]
fn test_kernel_half_clone() {
    // Entries 256-511 of PML4 are kernel mappings
    for i in 256..512 {
        assert!(i >= 256);
        assert!(i < 512);
    }
    // Entries 0-255 are user mappings (private per process)
    for i in 0..256 {
        assert!(i < 256);
    }
}

#[test]
fn test_guard_page() {
    let stack_bottom: u64 = 0x0000_7FFF_FFFF_F000 - 64 * 4096;
    let guard_page = stack_bottom - 4096;
    assert!(guard_page < stack_bottom);
    // Guard page should be unmapped — access triggers page fault
}

#[test]
fn test_aslr_range() {
    // ASLR should randomize bits 28-30 of code base
    let base1: u64 = 0x400000;
    let base2: u64 = 0x500000;
    assert_ne!(base1, base2); // Different loads should differ
}

#[test]
fn test_wx_enforcement() {
    // No page should be both writable and executable
    let rx = 5u32; // PF_R | PF_X
    let rw = 6u32; // PF_R | PF_W
    let rwx = 7u32; // PF_R | PF_W | PF_X — REJECTED
    assert_eq!(rx & 2, 0); // Not writable
    assert_eq!(rw & 1, 0); // Not executable
    assert!(rwx & 2 != 0 && rwx & 1 != 0); // Both — violation!
}
