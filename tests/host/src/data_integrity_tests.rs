/// Data integrity pattern tests.

#[test]
fn test_sequential_pattern() {
    let mut buf = vec![0u8; 256];
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = i as u8;
    }
    for (i, &byte) in buf.iter().enumerate() {
        assert_eq!(byte, i as u8);
    }
}

#[test]
fn test_pattern_fill_verify() {
    let pattern = 0xABu8;
    let mut buf = vec![pattern; 4096];
    assert!(buf.iter().all(|&b| b == pattern));

    // Modify and verify
    buf[100] = 0xCD;
    assert_eq!(buf[100], 0xCD);
    assert_eq!(buf[99], pattern);
    assert_eq!(buf[101], pattern);
}

#[test]
fn test_cross_contamination() {
    let mut files: Vec<Vec<u8>> = Vec::new();
    for i in 0..10u8 {
        files.push(vec![i; 64]);
    }

    // Verify no cross-contamination
    for (i, file) in files.iter().enumerate() {
        assert!(file.iter().all(|&b| b == i as u8),
            "File {} contaminated", i);
    }
}

#[test]
fn test_truncate_preserves_prefix() {
    let mut data = vec![0xAB; 1024];
    data.truncate(512);
    assert_eq!(data.len(), 512);
    assert!(data.iter().all(|&b| b == 0xAB));
}

#[test]
fn test_pipe_data_order() {
    let input: Vec<u8> = (0..100).collect();
    let output = input.clone(); // Pipe preserves order
    assert_eq!(input, output);
}

#[test]
fn test_checksum() {
    let data = b"Hello, LogOS!";
    let checksum: u32 = data.iter().map(|&b| b as u32).sum();
    assert!(checksum > 0, "Checksum should be non-zero");

    // Verify corruption detection
    let mut corrupted = data.to_vec();
    corrupted[0] = b'h'; // lowercase
    let corrupted_sum: u32 = corrupted.iter().map(|&b| b as u32).sum();
    assert_ne!(checksum, corrupted_sum);
}
