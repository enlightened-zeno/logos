// Host-side tests for LogOS data structures and algorithms.
// These run natively (not in QEMU) and test pure logic.

#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod errno_tests;
#[cfg(test)]
mod elf_tests;
#[cfg(test)]
mod signal_tests;
#[cfg(test)]
mod chacha20_tests;
#[cfg(test)]
mod fd_tests;
#[cfg(test)]
mod process_tests;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod vfs_tests;
#[cfg(test)]
mod syscall_tests;
#[cfg(test)]
mod timer_tests;
#[cfg(test)]
mod scheduler_tests;
#[cfg(test)]
mod pipe_tests;
#[cfg(test)]
mod block_cache_tests;
#[cfg(test)]
mod ext2_tests;
