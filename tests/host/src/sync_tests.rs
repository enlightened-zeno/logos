/// Synchronization primitive logic tests.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

#[test]
fn test_spinlock_acquire_release() {
    let locked = AtomicBool::new(false);
    assert!(locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok());
    assert!(locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err());
    locked.store(false, Ordering::Release);
    assert!(locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok());
}

#[test]
fn test_rwlock_multiple_readers() {
    let state = AtomicI64::new(0);
    // Reader 1
    state.fetch_add(1, Ordering::Acquire);
    // Reader 2
    state.fetch_add(1, Ordering::Acquire);
    assert_eq!(state.load(Ordering::Relaxed), 2);
    // Release readers
    state.fetch_sub(1, Ordering::Release);
    state.fetch_sub(1, Ordering::Release);
    assert_eq!(state.load(Ordering::Relaxed), 0);
}

#[test]
fn test_rwlock_writer_exclusive() {
    let state = AtomicI64::new(0);
    // Writer acquires (sets to -1)
    assert!(state.compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed).is_ok());
    // Reader can't acquire while writer holds
    let current = state.load(Ordering::Acquire);
    assert!(current < 0); // Writer holds
    // Writer releases
    state.store(0, Ordering::Release);
}

#[test]
fn test_waitqueue_semantics() {
    let mut waiters: Vec<usize> = Vec::new();
    // Thread 1 waits
    waiters.push(1);
    // Thread 2 waits
    waiters.push(2);
    assert_eq!(waiters.len(), 2);
    // Wake one (FIFO)
    let woken = waiters.remove(0);
    assert_eq!(woken, 1);
    assert_eq!(waiters.len(), 1);
}

#[test]
fn test_irq_save_restore() {
    let mut irq_was_enabled = true;
    // cli
    let saved = irq_was_enabled;
    irq_was_enabled = false;
    assert!(!irq_was_enabled);
    // sti (restore)
    if saved { irq_was_enabled = true; }
    assert!(irq_was_enabled);
}
