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
