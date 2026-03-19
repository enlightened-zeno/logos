extern crate alloc;

use crate::sync::SpinLock;
use alloc::collections::BTreeMap;

/// Process ID type.
pub type Pid = u64;

/// Process state in the process table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Zombie,
    Stopped,
}

/// Minimal process descriptor for the process table.
pub struct ProcessDesc {
    pub pid: Pid,
    pub ppid: Pid,
    pub pgid: Pid,
    pub sid: Pid,
    pub state: ProcessState,
    pub exit_code: i32,
    pub uid: u32,
    pub gid: u32,
}

/// Global process table.
struct ProcessTable {
    procs: BTreeMap<Pid, ProcessDesc>,
    next_pid: Pid,
}

static PROC_TABLE: SpinLock<Option<ProcessTable>> = SpinLock::new(None);

/// Initialize the process table with PID 1 (init/kernel).
pub fn init() {
    let mut table = ProcessTable {
        procs: BTreeMap::new(),
        next_pid: 2,
    };

    table.procs.insert(
        1,
        ProcessDesc {
            pid: 1,
            ppid: 0,
            pgid: 1,
            sid: 1,
            state: ProcessState::Running,
            exit_code: 0,
            uid: 0,
            gid: 0,
        },
    );

    *PROC_TABLE.lock() = Some(table);
}

/// Allocate a new PID.
pub fn alloc_pid() -> Pid {
    let mut guard = PROC_TABLE.lock();
    let table = guard.as_mut().expect("Process table not initialized");
    let pid = table.next_pid;
    table.next_pid += 1;
    pid
}

/// Register a new process in the table.
pub fn register(desc: ProcessDesc) {
    let mut guard = PROC_TABLE.lock();
    let table = guard.as_mut().expect("Process table not initialized");
    table.procs.insert(desc.pid, desc);
}

/// Get the process count.
pub fn count() -> usize {
    let guard = PROC_TABLE.lock();
    guard.as_ref().map(|t| t.procs.len()).unwrap_or(0)
}

/// Mark a process as zombie with the given exit code.
pub fn set_zombie(pid: Pid, exit_code: i32) {
    let mut guard = PROC_TABLE.lock();
    if let Some(table) = guard.as_mut() {
        if let Some(proc) = table.procs.get_mut(&pid) {
            proc.state = ProcessState::Zombie;
            proc.exit_code = exit_code;
        }
    }
}

/// Remove a zombie process (reap).
pub fn reap(pid: Pid) -> Option<i32> {
    let mut guard = PROC_TABLE.lock();
    let table = guard.as_mut()?;
    let proc = table.procs.get(&pid)?;
    if proc.state != ProcessState::Zombie {
        return None;
    }
    let exit_code = proc.exit_code;
    table.procs.remove(&pid);
    Some(exit_code)
}

/// Reparent all children of `parent_pid` to init (PID 1).
pub fn reparent_children(parent_pid: Pid) {
    let mut guard = PROC_TABLE.lock();
    if let Some(table) = guard.as_mut() {
        let children: alloc::vec::Vec<Pid> = table
            .procs
            .values()
            .filter(|p| p.ppid == parent_pid && p.pid != 1)
            .map(|p| p.pid)
            .collect();

        for child_pid in children {
            if let Some(child) = table.procs.get_mut(&child_pid) {
                child.ppid = 1;
            }
        }
    }
}

/// Find a zombie child of the given parent.
/// If `target_pid` is -1 (u64::MAX), returns any zombie child.
/// If `target_pid` is a specific PID, returns that child if zombie.
pub fn find_zombie_child(parent_pid: Pid, target_pid: u64) -> Option<(Pid, i32)> {
    let guard = PROC_TABLE.lock();
    let table = guard.as_ref()?;

    if target_pid == u64::MAX {
        // Wait for any child
        for proc in table.procs.values() {
            if proc.ppid == parent_pid && proc.state == ProcessState::Zombie {
                return Some((proc.pid, proc.exit_code));
            }
        }
    } else {
        // Wait for specific child
        let proc = table.procs.get(&target_pid)?;
        if proc.ppid == parent_pid && proc.state == ProcessState::Zombie {
            return Some((proc.pid, proc.exit_code));
        }
    }
    None
}

/// Check if a given PID has any children.
pub fn has_children(parent_pid: Pid) -> bool {
    let guard = PROC_TABLE.lock();
    guard
        .as_ref()
        .map(|t| t.procs.values().any(|p| p.ppid == parent_pid))
        .unwrap_or(false)
}

/// Get the parent PID of a process.
pub fn get_ppid(pid: Pid) -> Option<Pid> {
    let guard = PROC_TABLE.lock();
    guard.as_ref()?.procs.get(&pid).map(|p| p.ppid)
}

/// List all processes (for ps command).
pub fn list() -> alloc::vec::Vec<(Pid, Pid, ProcessState)> {
    let guard = PROC_TABLE.lock();
    guard
        .as_ref()
        .map(|t| t.procs.values().map(|p| (p.pid, p.ppid, p.state)).collect())
        .unwrap_or_default()
}
