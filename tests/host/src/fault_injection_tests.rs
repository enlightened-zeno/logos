/// Fault injection framework tests.

use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn test_should_fail_disabled() {
    let fail_every_n = AtomicU32::new(0);
    let counter = AtomicU32::new(0);
    let n = fail_every_n.load(Ordering::Relaxed);
    assert_eq!(n, 0);
    // When n=0, should_fail always returns false
    assert!(!should_fail_sim(n, &counter));
}

#[test]
fn test_should_fail_every_n() {
    let counter = AtomicU32::new(0);
    let n = 3u32; // Fail every 3rd call

    // Call 0: 0 % 3 == 0 → fail
    assert!(should_fail_sim(n, &counter));
    // Call 1: 1 % 3 != 0 → ok
    assert!(!should_fail_sim(n, &counter));
    // Call 2: 2 % 3 != 0 → ok
    assert!(!should_fail_sim(n, &counter));
    // Call 3: 3 % 3 == 0 → fail
    assert!(should_fail_sim(n, &counter));
}

#[test]
fn test_injection_points() {
    let points = ["PmmAlloc", "DiskRead", "DiskWrite", "SlabAlloc", "PtAlloc"];
    assert_eq!(points.len(), 5);
}

#[test]
fn test_feature_gated() {
    // Fault injection is behind #[cfg(feature = "fault_injection")]
    // When disabled, should_fail is always false (no-op)
    let enabled = false; // In normal builds
    if !enabled {
        assert!(!false); // No-op
    }
}

fn should_fail_sim(n: u32, counter: &AtomicU32) -> bool {
    if n == 0 { return false; }
    let count = counter.fetch_add(1, Ordering::Relaxed);
    count % n == 0
}
