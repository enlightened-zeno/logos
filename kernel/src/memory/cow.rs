//! Copy-on-Write frame reference counting.
//!
//! Tracks how many address spaces share each physical frame. When a write
//! fault occurs on a shared frame, the fault handler copies the frame and
//! gives the faulting process a private copy.

extern crate alloc;

use crate::memory::addr::PhysFrame;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;

/// Global COW reference count table.
static COW_REFCOUNTS: SpinLock<Option<BTreeMap<u64, u32>>> = SpinLock::new(None);

/// Initialize the COW reference tracking.
pub fn init() {
    *COW_REFCOUNTS.lock() = Some(BTreeMap::new());
}

/// Increment the reference count for a frame. Returns the new count.
pub fn inc_ref(frame: PhysFrame) -> u32 {
    let mut guard = COW_REFCOUNTS.lock();
    let map = guard.as_mut().expect("COW not initialized");
    let entry = map.entry(frame.start_address().as_u64()).or_insert(0);
    *entry += 1;
    *entry
}

/// Decrement the reference count for a frame. Returns the new count.
/// If count reaches 0, the entry is removed.
pub fn dec_ref(frame: PhysFrame) -> u32 {
    let mut guard = COW_REFCOUNTS.lock();
    let map = guard.as_mut().expect("COW not initialized");
    let addr = frame.start_address().as_u64();
    if let Some(count) = map.get_mut(&addr) {
        *count -= 1;
        let result = *count;
        if result == 0 {
            map.remove(&addr);
        }
        result
    } else {
        0
    }
}

/// Get the current reference count for a frame.
pub fn ref_count(frame: PhysFrame) -> u32 {
    let guard = COW_REFCOUNTS.lock();
    guard
        .as_ref()
        .and_then(|map| map.get(&frame.start_address().as_u64()).copied())
        .unwrap_or(0)
}

/// Check if a frame is shared (ref count > 1).
pub fn is_shared(frame: PhysFrame) -> bool {
    ref_count(frame) > 1
}
