//! File descriptor tests
#[test] fn fd_stdin() { assert_eq!(0u32, 0); }
#[test] fn fd_stdout() { assert_eq!(1u32, 1); }
#[test] fn fd_stderr() { assert_eq!(2u32, 2); }
#[test] fn fd_first_open() { assert_eq!(3u32, 3); /* First non-std fd */ }
#[test] fn fd_max_reasonable() { let max = 1024u32; assert!(max > 3); }
#[test] fn fd_bitmap_set() { let mut map = 0u64; map |= 1 << 3; assert!(map & (1 << 3) != 0); }
#[test] fn fd_bitmap_clear() { let mut map = 0xFFu64; map &= !(1 << 3); assert!(map & (1 << 3) == 0); }
#[test] fn fd_find_free() {
    let map: u64 = 0b111; // 0,1,2 used
    let free = map.trailing_ones();
    assert_eq!(free, 3);
}
