// Memory subsystem types have methods used across phases.
// Allow dead code to avoid warnings for not-yet-connected paths.
#[allow(dead_code)]
pub mod addr;
#[allow(dead_code)]
pub mod heap;
#[allow(dead_code)]
pub mod oom;
#[allow(dead_code)]
pub mod paging;
#[allow(dead_code)]
pub mod pmm;
pub mod slab;
#[allow(dead_code)]
pub mod vmm;
