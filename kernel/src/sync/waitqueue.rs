extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

/// A wait queue for blocking threads until a condition is met.
///
/// Threads call `wait()` to block and `wake_one()` or `wake_all()`
/// to unblock waiting threads.
pub struct WaitQueue {
    inner: crate::sync::SpinLock<VecDeque<Arc<AtomicBool>>>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            inner: crate::sync::SpinLock::new(VecDeque::new()),
        }
    }

    /// Block the current task until woken.
    pub fn wait(&self) {
        let flag = Arc::new(AtomicBool::new(false));
        {
            let mut queue = self.inner.lock();
            queue.push_back(flag.clone());
        }

        // Spin-wait with HLT (will be replaced with proper scheduler
        // integration when processes block on I/O)
        while !flag.load(Ordering::Acquire) {
            crate::arch::x86_64::cpu::hlt();
        }
    }

    /// Wake one waiting thread.
    pub fn wake_one(&self) {
        let mut queue = self.inner.lock();
        if let Some(flag) = queue.pop_front() {
            flag.store(true, Ordering::Release);
        }
    }

    /// Wake all waiting threads.
    pub fn wake_all(&self) {
        let mut queue = self.inner.lock();
        while let Some(flag) = queue.pop_front() {
            flag.store(true, Ordering::Release);
        }
    }

    /// Number of threads currently waiting.
    pub fn waiters(&self) -> usize {
        self.inner.lock().len()
    }
}
