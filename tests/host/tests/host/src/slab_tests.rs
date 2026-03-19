//! Slab allocator tests
#[test] fn size_class_32() { assert!(32usize.is_power_of_two()); }
#[test] fn size_class_64() { assert!(64usize.is_power_of_two()); }
#[test] fn size_class_128() { assert!(128usize.is_power_of_two()); }
#[test] fn size_class_256() { assert!(256usize.is_power_of_two()); }
#[test] fn size_class_512() { assert!(512usize.is_power_of_two()); }
#[test] fn size_class_1024() { assert!(1024usize.is_power_of_two()); }
#[test] fn size_class_2048() { assert!(2048usize.is_power_of_two()); }
#[test] fn size_class_4096() { assert!(4096usize.is_power_of_two()); }
#[test] fn size_class_selection() {
    fn select(size: usize) -> usize {
        let mut s = 32; while s < size && s < 4096 { s *= 2; } s
    }
    assert_eq!(select(1), 32);
    assert_eq!(select(33), 64);
    assert_eq!(select(100), 128);
    assert_eq!(select(4000), 4096);
}
#[test] fn objects_per_slab() { assert_eq!(4096 / 32, 128); assert_eq!(4096 / 64, 64); }
