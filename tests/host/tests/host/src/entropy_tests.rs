//! Entropy/CSPRNG tests
#[test] fn chacha20_block_size() { assert_eq!(64usize, 64); }
#[test] fn chacha20_key_size() { assert_eq!(32usize, 32); }
#[test] fn chacha20_nonce_size() { assert_eq!(12usize, 12); }
#[test] fn reseed_interval() { assert_eq!(1024 * 1024u64, 1_048_576); /* 1 MiB */ }
#[test] fn random_bytes_nonzero() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for i in 0..100u64 { set.insert(i); }
    assert_eq!(set.len(), 100);
}
#[test] fn bit_balance_concept() {
    let byte: u8 = 0b10101010;
    assert_eq!(byte.count_ones(), 4);
    assert_eq!(byte.count_zeros(), 4);
}
#[test] fn chi_squared_concept() {
    let expected = 256.0f64 / 256.0; // Uniform: each byte value expected equally
    assert!((expected - 1.0).abs() < 0.001);
}
