use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// IRQ-saving spinlock.
///
/// Disables interrupts on acquire and restores them on release.
/// This prevents deadlocks from interrupt handlers trying to acquire
/// a lock already held by the interrupted code.
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: SpinLock provides mutual exclusion through its atomic lock.
unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock, disabling interrupts. Returns a guard that
    /// releases the lock and restores interrupt state on drop.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // Save and disable interrupts before spinning
        let irq_was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
        crate::arch::x86_64::cpu::cli();

        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin with a hint to reduce power/bus contention
            core::hint::spin_loop();
        }

        SpinLockGuard {
            lock: self,
            irq_was_enabled,
        }
    }

    /// Try to acquire the lock without blocking. Returns `None` if already held.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        let irq_was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
        crate::arch::x86_64::cpu::cli();

        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard {
                lock: self,
                irq_was_enabled,
            })
        } else {
            // Restore interrupt state since we didn't acquire
            if irq_was_enabled {
                crate::arch::x86_64::cpu::sti();
            }
            None
        }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    irq_was_enabled: bool,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: We hold the lock, so exclusive access is guaranteed.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: We hold the lock, so exclusive access is guaranteed.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        if self.irq_was_enabled {
            crate::arch::x86_64::cpu::sti();
        }
    }
}
