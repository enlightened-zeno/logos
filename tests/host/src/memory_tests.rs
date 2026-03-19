/// Memory subsystem logic tests.

#[test]
fn test_page_alignment() {
    let page_size: u64 = 4096;
    assert_eq!(0u64 & !(page_size - 1), 0);
    assert_eq!(4096u64 & !(page_size - 1), 4096);
    assert_eq!(4097u64 & !(page_size - 1), 4096);
    assert_eq!(8191u64 & !(page_size - 1), 4096);
    assert_eq!(8192u64 & !(page_size - 1), 8192);
}

#[test]
fn test_align_up() {
    let align = |addr: u64, alignment: u64| -> u64 {
        (addr + alignment - 1) & !(alignment - 1)
    };
    assert_eq!(align(0, 4096), 0);
    assert_eq!(align(1, 4096), 4096);
    assert_eq!(align(4095, 4096), 4096);
    assert_eq!(align(4096, 4096), 4096);
    assert_eq!(align(4097, 4096), 8192);
}

#[test]
fn test_frame_number() {
    let addr: u64 = 0x12345000;
    let frame = addr >> 12;
    assert_eq!(frame, 0x12345);
    assert_eq!(frame << 12, addr);
}

#[test]
fn test_canonical_address() {
    // Canonical: bits 48-63 must match bit 47
    let canonicalize = |addr: u64| -> u64 {
        ((addr as i64) << 16 >> 16) as u64
    };

    // User space (bit 47 = 0)
    assert_eq!(canonicalize(0x0000_7FFF_FFFF_F000), 0x0000_7FFF_FFFF_F000);
    // Kernel space (bit 47 = 1)
    assert_eq!(canonicalize(0xFFFF_8000_0000_0000), 0xFFFF_8000_0000_0000);
    // Non-canonical gets sign-extended
    assert_eq!(canonicalize(0x0000_8000_0000_0000), 0xFFFF_8000_0000_0000);
}

#[test]
fn test_page_table_indices() {
    let vaddr: u64 = 0xFFFF_FFFF_8000_0000;
    let p4 = ((vaddr >> 39) & 0x1FF) as usize;
    let p3 = ((vaddr >> 30) & 0x1FF) as usize;
    let p2 = ((vaddr >> 21) & 0x1FF) as usize;
    let p1 = ((vaddr >> 12) & 0x1FF) as usize;
    let offset = vaddr & 0xFFF;

    assert!(p4 < 512);
    assert!(p3 < 512);
    assert!(p2 < 512);
    assert!(p1 < 512);
    assert!(offset < 4096);
    assert_eq!(p4, 511); // Kernel higher-half
}

#[test]
fn test_phys_addr_bits() {
    // Physical addresses must have bits 52-63 zero
    let valid: u64 = 0x000F_FFFF_FFFF_F000;
    assert_eq!(valid & 0xFFF0_0000_0000_0000, 0);
    let invalid: u64 = 0x0010_0000_0000_0000;
    assert_ne!(invalid & 0xFFF0_0000_0000_0000, 0);
}

#[test]
fn test_zone_classification() {
    // DMA16: 0..16 MiB
    // DMA32: 16 MiB..4 GiB
    // Normal: 4 GiB+
    let classify = |addr: u64| -> &'static str {
        if addr < 0x100_0000 { "dma16" }
        else if addr < 0x1_0000_0000 { "dma32" }
        else { "normal" }
    };

    assert_eq!(classify(0), "dma16");
    assert_eq!(classify(0xFF_FFFF), "dma16");
    assert_eq!(classify(0x100_0000), "dma32");
    assert_eq!(classify(0xFFFF_FFFF), "dma32");
    assert_eq!(classify(0x1_0000_0000), "normal");
}

#[test]
fn test_slab_size_classes() {
    let classes = [32, 64, 128, 256, 512, 1024, 2048, 4096];

    // Each class should be a power of 2 or at least double previous
    for i in 1..classes.len() {
        assert!(classes[i] >= classes[i-1] * 2,
            "Class {} ({}) should be >= 2x class {} ({})",
            i, classes[i], i-1, classes[i-1]);
    }

    // Find the right class for a given size
    let find_class = |size: usize| -> usize {
        classes.iter().position(|&s| size <= s).unwrap_or(classes.len())
    };

    assert_eq!(find_class(1), 0);   // -> 32
    assert_eq!(find_class(32), 0);  // -> 32
    assert_eq!(find_class(33), 1);  // -> 64
    assert_eq!(find_class(4096), 7); // -> 4096
    assert_eq!(find_class(4097), 8); // -> fallback to heap
}

#[test]
fn test_heap_layout_constants() {
    let heap_start: u64 = 0xFFFF_C000_0000_0000;
    let heap_max: u64 = heap_start + 256 * 1024 * 1024; // 256 MiB

    assert!(heap_start > 0xFFFF_8000_0000_0000); // In kernel space
    assert!(heap_max > heap_start);
    assert!(heap_max - heap_start == 256 * 1024 * 1024);
}
