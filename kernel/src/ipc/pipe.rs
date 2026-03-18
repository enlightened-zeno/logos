extern crate alloc;

use crate::fs::vfs::{Inode, InodeType, Stat};
use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::sync::Arc;

const PIPE_BUF_SIZE: usize = 65536; // 64 KiB

struct PipeInner {
    buf: [u8; PIPE_BUF_SIZE],
    head: usize,
    tail: usize,
    count: usize,
    /// Number of open write ends. When 0, reads return EOF.
    writers: usize,
    /// Number of open read ends. When 0, writes return EPIPE.
    readers: usize,
}

/// Shared pipe state.
pub struct Pipe {
    inner: SpinLock<PipeInner>,
}

impl Pipe {
    /// Create a new pipe and return (read_end, write_end).
    pub fn create() -> (Arc<PipeReader>, Arc<PipeWriter>) {
        let pipe = Arc::new(Self {
            inner: SpinLock::new(PipeInner {
                buf: [0; PIPE_BUF_SIZE],
                head: 0,
                tail: 0,
                count: 0,
                writers: 1,
                readers: 1,
            }),
        });

        let reader = Arc::new(PipeReader { pipe: pipe.clone() });
        let writer = Arc::new(PipeWriter { pipe });

        (reader, writer)
    }
}

/// Read end of a pipe.
pub struct PipeReader {
    pipe: Arc<Pipe>,
}

impl Inode for PipeReader {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(Stat {
            inode: 0,
            inode_type: InodeType::Pipe,
            size: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            mode: 0o444,
            dev: 0,
            rdev: 0,
            blksize: PIPE_BUF_SIZE as u64,
            blocks: 0,
        })
    }

    fn inode_type(&self) -> InodeType {
        InodeType::Pipe
    }

    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        loop {
            let mut inner = self.pipe.inner.lock();

            if inner.count > 0 {
                let to_read = buf.len().min(inner.count);
                let mut tail = inner.tail;
                for byte in buf.iter_mut().take(to_read) {
                    *byte = inner.buf[tail];
                    tail = (tail + 1) % PIPE_BUF_SIZE;
                }
                inner.tail = tail;
                inner.count -= to_read;
                return Ok(to_read);
            }

            if inner.writers == 0 {
                return Ok(0); // EOF
            }

            // No data and writers exist — would block.
            // For now, spin-yield instead of true blocking.
            drop(inner);
            core::hint::spin_loop();
        }
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        self.pipe.inner.lock().readers -= 1;
    }
}

/// Write end of a pipe.
pub struct PipeWriter {
    pipe: Arc<Pipe>,
}

impl Inode for PipeWriter {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(Stat {
            inode: 0,
            inode_type: InodeType::Pipe,
            size: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            mode: 0o222,
            dev: 0,
            rdev: 0,
            blksize: PIPE_BUF_SIZE as u64,
            blocks: 0,
        })
    }

    fn inode_type(&self) -> InodeType {
        InodeType::Pipe
    }

    fn write(&self, _offset: u64, data: &[u8]) -> Result<usize, Errno> {
        let mut inner = self.pipe.inner.lock();

        if inner.readers == 0 {
            return Err(Errno::EPIPE);
        }

        let space = PIPE_BUF_SIZE - inner.count;
        let to_write = data.len().min(space);

        let mut head = inner.head;
        for &byte in data.iter().take(to_write) {
            inner.buf[head] = byte;
            head = (head + 1) % PIPE_BUF_SIZE;
        }
        inner.head = head;
        inner.count += to_write;

        Ok(to_write)
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        self.pipe.inner.lock().writers -= 1;
    }
}
