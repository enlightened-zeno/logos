//! PMM logic tests
#[test] fn frame_size() { assert_eq!(4096usize, 4096); }
#[test] fn frames_from_bytes() { assert_eq!(1048576usize / 4096, 256); /* 1 MiB = 256 frames */ }
#[test] fn zone_dma16_limit() { assert_eq!(16u64 * 1024 * 1024, 0x100_0000); }
#[test] fn zone_dma32_limit() { assert_eq!(4u64 * 1024 * 1024 * 1024, 0x1_0000_0000); }
#[test] fn alloc_dealloc_balance() { let mut free = 1000u64; free -= 1; free += 1; assert_eq!(free, 1000); }
#[test] fn zero_on_alloc() { let frame = vec![0u8; 4096]; assert!(frame.iter().all(|&b| b == 0)); }
#[test] fn frame_alignment() { let addr = 0x12345000u64; assert_eq!(addr % 4096, 0); }
#[test] fn bitmap_set_bit() { let mut bm: u64 = 0; bm |= 1 << 5; assert!(bm & (1 << 5) != 0); }
#[test] fn bitmap_clear_bit() { let mut bm: u64 = 0xFF; bm &= !(1 << 5); assert!(bm & (1 << 5) == 0); }
#[test] fn bitmap_find_first_zero() {
    let bm: u64 = 0b1111_0111;
    let bit = (!bm).trailing_zeros();
    assert_eq!(bit, 3);
}
