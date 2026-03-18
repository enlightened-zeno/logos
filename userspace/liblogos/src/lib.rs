//! liblogos — Userspace syscall library for LogOS.
//!
//! Provides Rust-safe wrappers around raw syscalls, a print! macro,
//! and a brk-based allocator for userspace programs.

#![no_std]

pub mod syscall;
pub mod io;
pub mod alloc;

/// Syscall numbers (matching kernel dispatch table).
pub mod nr {
    pub const SYS_READ: u64 = 0;
    pub const SYS_WRITE: u64 = 1;
    pub const SYS_OPEN: u64 = 2;
    pub const SYS_CLOSE: u64 = 3;
    pub const SYS_BRK: u64 = 12;
    pub const SYS_GETPID: u64 = 39;
    pub const SYS_FORK: u64 = 57;
    pub const SYS_EXECVE: u64 = 59;
    pub const SYS_EXIT: u64 = 60;
    pub const SYS_WAIT4: u64 = 61;
    pub const SYS_UNAME: u64 = 63;
    pub const SYS_EXIT_GROUP: u64 = 231;
}
