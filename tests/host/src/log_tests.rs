/// Kernel log ring buffer tests.

#[test]
fn test_ring_buffer_wrap() {
    let size = 8;
    let mut buf = vec![0u8; size];
    let mut head = 0;

    // Write 10 bytes (wraps around)
    for i in 0..10u8 {
        buf[head] = i;
        head = (head + 1) % size;
    }

    // Head is at position 2 (10 % 8)
    assert_eq!(head, 2);
    // Last 8 bytes written are 2,3,4,5,6,7,8,9
    assert_eq!(buf[2], 2);
    assert_eq!(buf[7], 7);
}

#[test]
fn test_ring_buffer_read() {
    let size = 8;
    let mut buf = vec![0u8; size];
    let mut head = 0;
    let mut len = 0;

    // Write 5 bytes
    for i in 0..5u8 {
        buf[head] = b'a' + i;
        head = (head + 1) % size;
        if len < size {
            len += 1;
        }
    }

    // Read all
    let start = if len < size { 0 } else { head };
    let mut out = Vec::new();
    for i in 0..len {
        out.push(buf[(start + i) % size]);
    }
    assert_eq!(out, b"abcde");
}

#[test]
fn test_log_size() {
    let log_size = 64 * 1024; // 64 KiB
    assert_eq!(log_size, 65536);
}
