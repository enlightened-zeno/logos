extern crate alloc;

use crate::arch::x86_64::context::{switch_context, CpuContext};
use crate::process::{Priority, Task, TaskId, TaskState};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

/// Multi-Level Feedback Queue scheduler.
///
/// 4 priority levels with increasing quanta (10/20/40/80ms).
/// Tasks that exhaust their quantum are demoted. Priority boost
/// every 1 second moves all tasks to the highest queue.
struct MlfqInner {
    queues: [VecDeque<usize>; Priority::LEVELS],
    tasks: Vec<Option<Task>>,
    current: usize,
    last_boost_tick: u64,
}

/// Raw spinlock that doesn't disable interrupts — we manage that manually
/// around context switches to avoid holding a lock across switches.
static SCHED_LOCK: AtomicBool = AtomicBool::new(false);
static mut SCHED: Option<MlfqInner> = None;

fn lock_sched() {
    while SCHED_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
}

fn unlock_sched() {
    SCHED_LOCK.store(false, Ordering::Release);
}

/// Initialize the scheduler with the boot task as the current task.
pub fn init() {
    let boot_task = Task::from_current();

    let mut inner = MlfqInner {
        queues: [
            VecDeque::new(),
            VecDeque::new(),
            VecDeque::new(),
            VecDeque::new(),
        ],
        tasks: Vec::new(),
        current: 0,
        last_boost_tick: 0,
    };

    inner.tasks.push(Some(boot_task));
    // SAFETY: Called once during single-threaded boot.
    unsafe {
        core::ptr::addr_of_mut!(SCHED).write(Some(inner));
    }

    crate::serial_println!("Scheduler: initialized (MLFQ, 4 levels)");
}

/// Spawn a new kernel task.
pub fn spawn(entry: fn() -> !) -> TaskId {
    let was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
    crate::arch::x86_64::cpu::cli();
    lock_sched();

    // SAFETY: Lock is held, interrupts disabled.
    let sched = unsafe {
        (*core::ptr::addr_of_mut!(SCHED))
            .as_mut()
            .expect("Scheduler not initialized")
    };

    let task = Task::new_kernel(entry);
    let id = task.id;
    let idx = sched.tasks.len();
    let priority = task.priority.0 as usize;
    sched.tasks.push(Some(task));
    sched.queues[priority].push_back(idx);

    unlock_sched();
    if was_enabled {
        crate::arch::x86_64::cpu::sti();
    }

    id
}

/// Called from the timer ISR on every tick.
pub fn timer_tick() {
    // Try to acquire — if held (mid-switch), skip this tick
    if SCHED_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    // SAFETY: Lock is held.
    let sched = match unsafe { (*core::ptr::addr_of_mut!(SCHED)).as_mut() } {
        Some(s) => s,
        None => {
            unlock_sched();
            return;
        }
    };

    let current_ticks = crate::arch::x86_64::apic::ticks();

    if current_ticks - sched.last_boost_tick >= 1000 {
        sched.last_boost_tick = current_ticks;
        boost_all(sched);
    }

    let current = sched.current;
    if let Some(task) = &mut sched.tasks[current] {
        task.total_ticks += 1;
        if task.ticks_remaining > 0 {
            task.ticks_remaining -= 1;
        }

        if task.ticks_remaining == 0 {
            let old_priority = task.priority;
            if old_priority.0 < (Priority::LEVELS - 1) as u8 {
                task.priority = Priority(old_priority.0 + 1);
            }
            task.ticks_remaining = task.priority.quantum_ms();
            task.state = TaskState::Ready;
            sched.queues[task.priority.0 as usize].push_back(current);

            do_switch(sched);
            // Lock is released inside do_switch
            return;
        }
    }

    unlock_sched();
}

/// Voluntarily yield the current task's remaining quantum.
pub fn yield_now() {
    let was_enabled = crate::arch::x86_64::cpu::interrupts_enabled();
    crate::arch::x86_64::cpu::cli();
    lock_sched();

    // SAFETY: Lock is held, interrupts disabled.
    let sched = match unsafe { (*core::ptr::addr_of_mut!(SCHED)).as_mut() } {
        Some(s) => s,
        None => {
            unlock_sched();
            if was_enabled {
                crate::arch::x86_64::cpu::sti();
            }
            return;
        }
    };

    let current = sched.current;
    if let Some(task) = &mut sched.tasks[current] {
        task.state = TaskState::Ready;
        task.ticks_remaining = task.priority.quantum_ms();
        sched.queues[task.priority.0 as usize].push_back(current);
    }

    do_switch(sched);
    // Lock released inside do_switch, we resume here after being switched back

    if was_enabled {
        crate::arch::x86_64::cpu::sti();
    }
}

/// Get the current task's ID.
pub fn current_task_id() -> TaskId {
    lock_sched();
    // SAFETY: Lock is held.
    let sched = unsafe {
        (*core::ptr::addr_of!(SCHED))
            .as_ref()
            .expect("Scheduler not initialized")
    };
    let id = sched.tasks[sched.current]
        .as_ref()
        .expect("Current task is None")
        .id;
    unlock_sched();
    id
}

/// Pick the next task, release the lock, then context switch.
///
/// The lock MUST be held on entry. It is released before the switch
/// so the new task can acquire it without deadlock.
fn do_switch(sched: &mut MlfqInner) {
    let next_idx = sched
        .queues
        .iter_mut()
        .find_map(|q| q.pop_front())
        .unwrap_or(sched.current);

    if next_idx == sched.current {
        if let Some(task) = &mut sched.tasks[sched.current] {
            task.state = TaskState::Running;
        }
        unlock_sched();
        return;
    }

    let old = sched.current;
    sched.current = next_idx;

    if let Some(task) = &mut sched.tasks[next_idx] {
        task.state = TaskState::Running;
    }

    let old_ctx = &mut sched.tasks[old].as_mut().unwrap().context as *mut CpuContext;
    let new_ctx = &sched.tasks[next_idx].as_ref().unwrap().context as *const CpuContext;

    // Release lock BEFORE switch — the new task may need the scheduler
    unlock_sched();

    // SAFETY: Both contexts are valid. Lock is released. The old task
    // is suspended here; when it resumes, execution continues after this call.
    unsafe {
        switch_context(old_ctx, new_ctx);
    }
    // Resumed: someone switched back to us.
}

fn boost_all(sched: &mut MlfqInner) {
    for level in 1..Priority::LEVELS {
        while let Some(idx) = sched.queues[level].pop_front() {
            if let Some(task) = &mut sched.tasks[idx] {
                task.priority = Priority::HIGHEST;
                task.ticks_remaining = Priority::HIGHEST.quantum_ms();
            }
            sched.queues[0].push_back(idx);
        }
    }
}
