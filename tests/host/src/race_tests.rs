/// Race condition and concurrency logic tests.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn test_atomic_counter_consistency() {
    let counter = Arc::new(AtomicU64::new(0));
    let threads: Vec<_> = (0..4).map(|_| {
        let c = counter.clone();
        std::thread::spawn(move || {
            for _ in 0..10000 {
                c.fetch_add(1, Ordering::Relaxed);
            }
        })
    }).collect();
    for t in threads { t.join().unwrap(); }
    assert_eq!(counter.load(Ordering::Relaxed), 40000);
}

#[test]
fn test_spinlock_mutual_exclusion() {
    use std::sync::atomic::AtomicBool;
    let lock = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let threads: Vec<_> = (0..4).map(|_| {
        let l = lock.clone();
        let c = counter.clone();
        std::thread::spawn(move || {
            for _ in 0..1000 {
                while l.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
                    std::hint::spin_loop();
                }
                let val = c.load(Ordering::Relaxed);
                c.store(val + 1, Ordering::Relaxed);
                l.store(false, Ordering::Release);
            }
        })
    }).collect();
    for t in threads { t.join().unwrap(); }
    assert_eq!(counter.load(Ordering::Relaxed), 4000);
}

#[test]
fn test_compare_exchange_semantics() {
    let val = AtomicU64::new(42);
    assert!(val.compare_exchange(42, 100, Ordering::SeqCst, Ordering::SeqCst).is_ok());
    assert_eq!(val.load(Ordering::Relaxed), 100);
    assert!(val.compare_exchange(42, 200, Ordering::SeqCst, Ordering::SeqCst).is_err());
    assert_eq!(val.load(Ordering::Relaxed), 100); // Unchanged
}

#[test]
fn test_fence_ordering() {
    use std::sync::atomic::fence;
    let a = AtomicU64::new(0);
    let b = AtomicU64::new(0);
    a.store(1, Ordering::Relaxed);
    fence(Ordering::SeqCst);
    b.store(1, Ordering::Relaxed);
    // After fence, both should be visible
    assert_eq!(a.load(Ordering::Relaxed), 1);
    assert_eq!(b.load(Ordering::Relaxed), 1);
}
