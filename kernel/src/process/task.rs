extern crate alloc;

use crate::arch::x86_64::context::CpuContext;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Task execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Dead,
}

/// MLFQ priority level (0 = highest, 3 = lowest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority(pub u8);

impl Priority {
    pub const HIGHEST: Self = Self(0);
    pub const HIGH: Self = Self(1);
    pub const NORMAL: Self = Self(2);
    pub const LOW: Self = Self(3);
    pub const LEVELS: usize = 4;

    /// Time quantum in milliseconds for this priority level.
    pub fn quantum_ms(self) -> u64 {
        match self.0 {
            0 => 10,
            1 => 20,
            2 => 40,
            _ => 80,
        }
    }
}

const KERNEL_STACK_SIZE: usize = 4096 * 8; // 32 KiB

/// A kernel task (thread of execution).
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub priority: Priority,
    pub context: CpuContext,
    /// Ticks remaining in current quantum.
    pub ticks_remaining: u64,
    /// Total ticks this task has run.
    pub total_ticks: u64,
    /// Kernel stack allocation (owned).
    _kernel_stack: Box<[u8]>,
}

impl Task {
    /// Create a new kernel task that will start executing at `entry_point`.
    pub fn new_kernel(entry_point: fn() -> !) -> Self {
        let id = TaskId::new();
        let stack = alloc::vec![0u8; KERNEL_STACK_SIZE].into_boxed_slice();
        let stack_top = stack.as_ptr() as u64 + KERNEL_STACK_SIZE as u64;

        // Read current CR3 — kernel tasks share the kernel address space
        let cr3: u64;
        // SAFETY: Reading CR3 is always safe.
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
        }

        // Set up the stack so that when switch_context does `ret`, it jumps
        // to entry_point. We push the entry address onto the stack.
        let stack_ptr = stack_top - 8;
        // SAFETY: We own this stack and it's large enough.
        unsafe {
            *(stack_ptr as *mut u64) = entry_point as *const () as u64;
        }

        let mut ctx = CpuContext::new_kernel(entry_point as *const () as u64, stack_top, cr3);
        ctx.rsp = stack_ptr;

        let priority = Priority::HIGHEST;
        Self {
            id,
            state: TaskState::Ready,
            priority,
            context: ctx,
            ticks_remaining: priority.quantum_ms(),
            total_ticks: 0,
            _kernel_stack: stack,
        }
    }

    /// Create a task representing the current (boot) execution context.
    /// Context will be filled in on first switch away.
    pub fn from_current() -> Self {
        let id = TaskId::new();
        let cr3: u64;
        // SAFETY: Reading CR3 is always safe.
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
        }

        let mut ctx = CpuContext::empty();
        ctx.cr3 = cr3;

        let priority = Priority::NORMAL;
        Self {
            id,
            state: TaskState::Running,
            priority,
            context: ctx,
            ticks_remaining: priority.quantum_ms(),
            total_ticks: 0,
            _kernel_stack: Box::new([]),
        }
    }
}
