// Host-side tests for LogOS
// These test pure logic that can run on the host without the kernel.

mod addr_tests;
mod errno_tests;
mod path_tests;
mod elf_tests;
mod syscall_num_tests;
mod timer_tests;
mod signal_tests;
mod fd_tests;
mod cache_tests;
mod sync_tests;
mod pmm_tests;
mod slab_tests;
mod scheduler_tests;
mod tty_tests;
mod keyboard_tests;
mod pci_tests;
mod entropy_tests;
mod oom_tests;
mod process_tests;
mod boot_tests;
mod shell_tests;
mod coreutils_tests;
mod interop_tests;
mod fault_tests;
mod perf_tests;
mod leak_tests;
mod data_tests;
mod race_tests;
mod recovery_tests;
mod soak_tests;
mod cross_tests;
