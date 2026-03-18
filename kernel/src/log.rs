use core::sync::atomic::{AtomicUsize, Ordering};

/// Kernel log ring buffer.
/// Stores all serial output for retrieval via `dmesg`.
const LOG_SIZE: usize = 64 * 1024; // 64 KiB

static mut LOG_BUF: [u8; LOG_SIZE] = [0; LOG_SIZE];
static LOG_HEAD: AtomicUsize = AtomicUsize::new(0);
static LOG_LEN: AtomicUsize = AtomicUsize::new(0);

/// Append bytes to the kernel log.
pub fn append(data: &[u8]) {
    for &byte in data {
        let head = LOG_HEAD.load(Ordering::Relaxed);
        // SAFETY: LOG_BUF is only written one byte at a time with atomic index tracking.
        // Concurrent reads may see partial data but won't corrupt memory.
        unsafe {
            LOG_BUF[head] = byte;
        }
        LOG_HEAD.store((head + 1) % LOG_SIZE, Ordering::Relaxed);
        let len = LOG_LEN.load(Ordering::Relaxed);
        if len < LOG_SIZE {
            LOG_LEN.store(len + 1, Ordering::Relaxed);
        }
    }
}

/// Read the entire kernel log into a buffer. Returns bytes written.
pub fn read(buf: &mut [u8]) -> usize {
    let len = LOG_LEN.load(Ordering::Relaxed);
    let head = LOG_HEAD.load(Ordering::Relaxed);

    let to_copy = buf.len().min(len);
    let start = if len < LOG_SIZE {
        0
    } else {
        head // Wrapped — oldest data starts at head
    };

    for (i, byte) in buf.iter_mut().take(to_copy).enumerate() {
        let idx = (start + i) % LOG_SIZE;
        // SAFETY: idx is within LOG_BUF bounds.
        *byte = unsafe { LOG_BUF[idx] };
    }

    to_copy
}
