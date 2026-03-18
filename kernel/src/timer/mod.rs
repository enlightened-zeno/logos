extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 4-level hierarchical timer wheel.
/// O(1) insert and O(1) per-tick advance.
const WHEEL_SIZE: [usize; 4] = [256, 64, 64, 64];
const WHEEL_BITS: [u32; 4] = [8, 6, 6, 6];

type TimerCallback = Box<dyn FnOnce() + Send>;

struct TimerEntry {
    deadline: u64,
    callback: TimerCallback,
    next: Option<Box<TimerEntry>>,
}

struct WheelLevel {
    slots: Vec<Option<Box<TimerEntry>>>,
}

impl WheelLevel {
    fn new(size: usize) -> Self {
        let mut slots = Vec::with_capacity(size);
        slots.resize_with(size, || None);
        Self { slots }
    }
}

struct TimerWheelInner {
    levels: [WheelLevel; 4],
    pending_count: u64,
}

// Raw atomic lock — no interrupt toggling, safe for ISR use.
static WHEEL_LOCK: AtomicBool = AtomicBool::new(false);
static mut WHEEL: Option<TimerWheelInner> = None;
static CURRENT_TICK: AtomicU64 = AtomicU64::new(0);

fn lock_wheel() -> bool {
    WHEEL_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
}

fn unlock_wheel() {
    WHEEL_LOCK.store(false, Ordering::Release);
}

/// Initialize the timer wheel.
pub fn init() {
    let inner = TimerWheelInner {
        levels: [
            WheelLevel::new(WHEEL_SIZE[0]),
            WheelLevel::new(WHEEL_SIZE[1]),
            WheelLevel::new(WHEEL_SIZE[2]),
            WheelLevel::new(WHEEL_SIZE[3]),
        ],
        pending_count: 0,
    };
    // SAFETY: Called once during single-threaded boot.
    unsafe {
        core::ptr::addr_of_mut!(WHEEL).write(Some(inner));
    }
    crate::serial_println!("Timer wheel: initialized (4 levels)");
}

/// Schedule a callback to fire after `delay_ms` milliseconds.
pub fn schedule(delay_ms: u64, callback: impl FnOnce() + Send + 'static) {
    // Disable interrupts while holding the wheel lock to prevent the
    // timer ISR from trying to acquire the same lock (deadlock).
    let was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
    crate::arch::x86_64::cpu::cli();

    while !lock_wheel() {
        core::hint::spin_loop();
    }

    // SAFETY: Lock is held.
    let wheel = unsafe { (*core::ptr::addr_of_mut!(WHEEL)).as_mut().unwrap() };
    let tick = CURRENT_TICK.load(Ordering::Relaxed);
    let deadline = tick + delay_ms;
    let entry = Box::new(TimerEntry {
        deadline,
        callback: Box::new(callback),
        next: None,
    });
    insert_entry(wheel, entry, tick);
    wheel.pending_count += 1;

    unlock_wheel();
    if was_enabled {
        crate::arch::x86_64::cpu::sti();
    }
}

/// Advance the timer wheel by one tick. Called from the APIC timer ISR.
pub fn tick() {
    // Always increment tick counter, even if we can't process callbacks.
    let tick = CURRENT_TICK.fetch_add(1, Ordering::Relaxed) + 1;

    if !lock_wheel() {
        return; // Callbacks will fire on a future tick when the slot is checked.
    }

    // SAFETY: Lock is held.
    let wheel = match unsafe { (*core::ptr::addr_of_mut!(WHEEL)).as_mut() } {
        Some(w) => w,
        None => {
            unlock_wheel();
            return;
        }
    };

    let slot_idx = (tick as usize) & (WHEEL_SIZE[0] - 1);
    let mut entry_opt = wheel.levels[0].slots[slot_idx].take();

    let mut to_fire: Vec<TimerCallback> = Vec::new();

    while let Some(mut entry) = entry_opt {
        let next = entry.next.take();
        if entry.deadline <= tick {
            to_fire.push(entry.callback);
            wheel.pending_count -= 1;
        } else {
            insert_entry(wheel, entry, tick);
        }
        entry_opt = next;
    }

    if slot_idx == 0 {
        cascade(wheel, 1, tick);
    }

    unlock_wheel();

    for cb in to_fire {
        cb();
    }
}

/// Get current tick count.
pub fn current_tick() -> u64 {
    CURRENT_TICK.load(Ordering::Relaxed)
}

/// Get number of pending timers.
pub fn pending_count() -> u64 {
    let was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
    crate::arch::x86_64::cpu::cli();

    if !lock_wheel() {
        if was_enabled {
            crate::arch::x86_64::cpu::sti();
        }
        return 0;
    }
    // SAFETY: Lock is held.
    let count = unsafe { (*core::ptr::addr_of_mut!(WHEEL)).as_ref() }
        .map(|w| w.pending_count)
        .unwrap_or(0);
    unlock_wheel();
    if was_enabled {
        crate::arch::x86_64::cpu::sti();
    }
    count
}

/// Sleep for approximately `ms` milliseconds (busy-wait with HLT).
pub fn sleep_ms(ms: u64) {
    use alloc::sync::Arc;

    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    schedule(ms, move || {
        done_clone.store(true, Ordering::Release);
    });

    while !done.load(Ordering::Acquire) {
        crate::arch::x86_64::cpu::hlt();
    }
}

fn insert_entry(wheel: &mut TimerWheelInner, entry: Box<TimerEntry>, current: u64) {
    let delta = entry.deadline.saturating_sub(current);

    let (level, slot) = if delta < WHEEL_SIZE[0] as u64 {
        (0, (entry.deadline as usize) & (WHEEL_SIZE[0] - 1))
    } else if delta < (WHEEL_SIZE[0] as u64) * (WHEEL_SIZE[1] as u64) {
        (
            1,
            ((entry.deadline >> WHEEL_BITS[0]) as usize) & (WHEEL_SIZE[1] - 1),
        )
    } else if delta < (WHEEL_SIZE[0] as u64) * (WHEEL_SIZE[1] as u64) * (WHEEL_SIZE[2] as u64) {
        (
            2,
            ((entry.deadline >> (WHEEL_BITS[0] + WHEEL_BITS[1])) as usize) & (WHEEL_SIZE[2] - 1),
        )
    } else {
        (
            3,
            ((entry.deadline >> (WHEEL_BITS[0] + WHEEL_BITS[1] + WHEEL_BITS[2])) as usize)
                & (WHEEL_SIZE[3] - 1),
        )
    };

    let mut entry = entry;
    entry.next = wheel.levels[level].slots[slot].take();
    wheel.levels[level].slots[slot] = Some(entry);
}

fn cascade(wheel: &mut TimerWheelInner, level: usize, tick: u64) {
    if level >= 4 {
        return;
    }

    let shift: u32 = WHEEL_BITS[..level].iter().sum();
    let slot_idx = ((tick >> shift) as usize) & (WHEEL_SIZE[level] - 1);

    let mut entry_opt = wheel.levels[level].slots[slot_idx].take();
    while let Some(mut entry) = entry_opt {
        let next = entry.next.take();
        insert_entry(wheel, entry, tick);
        entry_opt = next;
    }

    if slot_idx == 0 && level + 1 < 4 {
        cascade(wheel, level + 1, tick);
    }
}
