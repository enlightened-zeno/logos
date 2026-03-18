#[cfg(feature = "fault_injection")]
use core::sync::atomic::{AtomicU32, Ordering};

/// Fault injection points. Enabled via `--features fault_injection`.
/// Each point has a counter: fail every Nth call (0 = disabled).

#[cfg(feature = "fault_injection")]
pub static PMM_FAIL_EVERY_N: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "fault_injection")]
pub static DISK_READ_FAIL_EVERY_N: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "fault_injection")]
pub static DISK_WRITE_FAIL_EVERY_N: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "fault_injection")]
pub static SLAB_FAIL_EVERY_N: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "fault_injection")]
pub static PT_ALLOC_FAIL_EVERY_N: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "fault_injection")]
static COUNTERS: [AtomicU32; 5] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum InjectionPoint {
    PmmAlloc = 0,
    DiskRead = 1,
    DiskWrite = 2,
    SlabAlloc = 3,
    PtAlloc = 4,
}

/// Check if this call should fail. Returns true if fault should be injected.
#[cfg(feature = "fault_injection")]
pub fn should_fail(point: InjectionPoint) -> bool {
    let n = match point {
        InjectionPoint::PmmAlloc => PMM_FAIL_EVERY_N.load(Ordering::Relaxed),
        InjectionPoint::DiskRead => DISK_READ_FAIL_EVERY_N.load(Ordering::Relaxed),
        InjectionPoint::DiskWrite => DISK_WRITE_FAIL_EVERY_N.load(Ordering::Relaxed),
        InjectionPoint::SlabAlloc => SLAB_FAIL_EVERY_N.load(Ordering::Relaxed),
        InjectionPoint::PtAlloc => PT_ALLOC_FAIL_EVERY_N.load(Ordering::Relaxed),
    };
    if n == 0 {
        return false;
    }
    let count = COUNTERS[point as usize].fetch_add(1, Ordering::Relaxed);
    count % n == 0
}

/// No-op when fault injection is disabled.
#[cfg(not(feature = "fault_injection"))]
#[inline(always)]
pub fn should_fail(_point: InjectionPoint) -> bool {
    false
}
