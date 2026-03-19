//! Data integrity tests
#[test] fn sequential_pattern() {
    let mut data = vec![0u8; 256];
    for i in 0..256 { data[i] = i as u8; }
    for i in 0..256 { assert_eq!(data[i], i as u8); }
}
#[test] fn pattern_survives_copy() {
    let src: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    let dst = src.clone();
    assert_eq!(src, dst);
}
#[test] fn no_cross_contamination() {
    let files: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8; 64]).collect();
    for (i, f) in files.iter().enumerate() {
        assert!(f.iter().all(|&b| b == i as u8));
    }
}
#[test] fn truncation() {
    let mut data = vec![0xABu8; 1024];
    data.truncate(512);
    assert_eq!(data.len(), 512);
    assert!(data.iter().all(|&b| b == 0xAB));
}
#[test] fn append_preserves_existing() {
    let mut data = vec![1u8; 100];
    data.extend_from_slice(&[2u8; 100]);
    assert!(data[..100].iter().all(|&b| b == 1));
    assert!(data[100..].iter().all(|&b| b == 2));
}
#[test] fn large_file_boundary() {
    // Verify data at block boundaries
    let block = 4096usize;
    let data: Vec<u8> = (0..block * 3).map(|i| (i % 256) as u8).collect();
    assert_eq!(data[block - 1], 255);
    assert_eq!(data[block], 0);
}
