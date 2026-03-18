use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicI64, Ordering};

use super::waitqueue::WaitQueue;

/// Reader-writer lock with writer preference.
///
/// Multiple readers can hold the lock concurrently, but a writer
/// gets exclusive access. Writers are preferred: when a writer is
/// waiting, new readers queue behind it.
pub struct RwLock<T> {
    /// Positive = reader count, -1 = write-locked, 0 = unlocked.
    state: AtomicI64,
    write_waiters: WaitQueue,
    read_waiters: WaitQueue,
    data: UnsafeCell<T>,
}

// SAFETY: RwLock provides proper synchronization.
unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicI64::new(0),
            write_waiters: WaitQueue::new(),
            read_waiters: WaitQueue::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state >= 0 {
                // No writer — try to increment reader count
                if self
                    .state
                    .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return RwLockReadGuard { lock: self };
                }
            } else {
                // Writer holds lock — wait
                self.read_waiters.wait();
            }
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        loop {
            if self
                .state
                .compare_exchange_weak(0, -1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return RwLockWriteGuard { lock: self };
            }
            self.write_waiters.wait();
        }
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: Readers have shared access.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let prev = self.lock.state.fetch_sub(1, Ordering::Release);
        if prev == 1 {
            // Last reader — wake a waiting writer
            self.lock.write_waiters.wake_one();
        }
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: Writer has exclusive access.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: Writer has exclusive access.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
        // Wake all waiting readers, then one writer
        self.lock.read_waiters.wake_all();
        self.lock.write_waiters.wake_one();
    }
}
