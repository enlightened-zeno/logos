//! Synchronization primitive tests
use std::sync::{Arc, Mutex};
use std::thread;

#[test] fn mutex_basic() {
    let m = Mutex::new(42);
    { let mut g = m.lock().unwrap(); *g = 100; }
    assert_eq!(*m.lock().unwrap(), 100);
}
#[test] fn mutex_concurrent() {
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];
    for _ in 0..4 {
        let c = counter.clone();
        handles.push(thread::spawn(move || { for _ in 0..1000 { *c.lock().unwrap() += 1; } }));
    }
    for h in handles { h.join().unwrap(); }
    assert_eq!(*counter.lock().unwrap(), 4000);
}
#[test] fn spinlock_concept() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let locked = AtomicBool::new(false);
    assert!(!locked.swap(true, Ordering::Acquire));
    assert!(locked.swap(true, Ordering::Acquire)); // Already locked
    locked.store(false, Ordering::Release);
    assert!(!locked.swap(true, Ordering::Acquire));
}
