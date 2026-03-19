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
#[cfg(test)]
mod boot_tests;
#[cfg(test)]
mod oom_tests;
#[cfg(test)]
mod power_tests;
#[cfg(test)]
mod smp_tests;
#[cfg(test)]
mod tty_tests;
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod shm_tests;
#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod panic_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod data_integrity_tests;
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod sync_tests;
#[cfg(test)]
mod virtio_tests;
#[cfg(test)]
mod pci_tests;
#[cfg(test)]
mod acpi_tests;
#[cfg(test)]
mod coreutils_tests;
#[cfg(test)]
mod fault_injection_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod race_tests;
#[cfg(test)]
mod liblogos_tests;
#[cfg(test)]
mod address_space_tests;
#[cfg(test)]
mod leak_tests;
#[cfg(test)]
mod soak_tests;
#[cfg(test)]
mod xsys_tests;
#[cfg(test)]
mod framebuffer_tests;
#[cfg(test)]
mod serial_tests;
#[cfg(test)]
mod perf_tests;
#[cfg(test)]
mod interop_tests;
#[cfg(test)]
mod devfs_tests;
#[cfg(test)]
mod procfs_tests;
#[cfg(test)]
mod tmpfs_tests;
#[cfg(test)]
mod util_tests;
#[cfg(test)]
mod io_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod pmm_spec_tests;
#[cfg(test)]
mod vmm_spec_tests;
#[cfg(test)]
mod syscall_spec_tests;
#[cfg(test)]
mod sched_spec_tests;
#[cfg(test)]
mod fs_spec_tests;
#[cfg(test)]
mod ipc_spec_tests;
#[cfg(test)]
mod remaining_spec_tests;
#[cfg(test)]
mod final_tests;
