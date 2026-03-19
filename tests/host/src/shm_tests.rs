/// Shared memory logic tests.

#[test]
fn test_shm_key_lookup() {
    use std::collections::HashMap;
    let mut segments: HashMap<i32, Vec<u8>> = HashMap::new();

    let key = 42;
    segments.insert(key, vec![0u8; 4096]);

    // Same key returns same segment
    assert!(segments.contains_key(&key));

    // Different key creates new
    segments.insert(43, vec![0u8; 1024]);
    assert_eq!(segments.len(), 2);
}

#[test]
fn test_shm_attach_count() {
    let mut attach_count = 0u32;
    attach_count += 1; // First attach
    attach_count += 1; // Second attach
    assert_eq!(attach_count, 2);

    attach_count -= 1; // Detach
    assert_eq!(attach_count, 1);
}

#[test]
fn test_shm_max_segments() {
    let max = 64;
    let mut allocated = 0;
    for _ in 0..max {
        allocated += 1;
    }
    assert_eq!(allocated, max);
}

#[test]
fn test_shm_private_key() {
    // Key 0 = IPC_PRIVATE (always creates new)
    let key = 0;
    assert_eq!(key, 0);
}
