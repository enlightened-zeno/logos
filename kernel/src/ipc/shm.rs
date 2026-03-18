extern crate alloc;

use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::vec;
use alloc::vec::Vec;

/// Maximum shared memory segments.
const MAX_SHM_SEGMENTS: usize = 64;

/// A shared memory segment.
struct ShmSegment {
    key: i32,
    size: usize,
    data: Vec<u8>,
    attach_count: usize,
}

struct ShmState {
    segments: Vec<Option<ShmSegment>>,
}

static SHM: SpinLock<Option<ShmState>> = SpinLock::new(None);

/// Initialize the shared memory subsystem.
pub fn init() {
    let mut segments = Vec::with_capacity(MAX_SHM_SEGMENTS);
    segments.resize_with(MAX_SHM_SEGMENTS, || None);
    *SHM.lock() = Some(ShmState { segments });
}

/// Create or get a shared memory segment. Returns segment ID.
pub fn shmget(key: i32, size: usize) -> Result<usize, Errno> {
    let mut guard = SHM.lock();
    let state = guard.as_mut().ok_or(Errno::ENOSYS)?;

    // Check if segment with this key exists
    if key != 0 {
        for (id, slot) in state.segments.iter().enumerate() {
            if let Some(seg) = slot {
                if seg.key == key {
                    return Ok(id);
                }
            }
        }
    }

    // Allocate new segment
    for (id, slot) in state.segments.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(ShmSegment {
                key,
                size,
                data: vec![0u8; size],
                attach_count: 0,
            });
            return Ok(id);
        }
    }

    Err(Errno::ENOMEM)
}

/// Attach to a shared memory segment. Returns a pointer to the data.
pub fn shmat(id: usize) -> Result<*mut u8, Errno> {
    let mut guard = SHM.lock();
    let state = guard.as_mut().ok_or(Errno::ENOSYS)?;

    let seg = state
        .segments
        .get_mut(id)
        .and_then(|s| s.as_mut())
        .ok_or(Errno::EINVAL)?;

    seg.attach_count += 1;
    Ok(seg.data.as_mut_ptr())
}

/// Detach from a shared memory segment.
pub fn shmdt(id: usize) -> Result<(), Errno> {
    let mut guard = SHM.lock();
    let state = guard.as_mut().ok_or(Errno::ENOSYS)?;

    let seg = state
        .segments
        .get_mut(id)
        .and_then(|s| s.as_mut())
        .ok_or(Errno::EINVAL)?;

    if seg.attach_count > 0 {
        seg.attach_count -= 1;
    }

    Ok(())
}
