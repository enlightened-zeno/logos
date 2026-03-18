extern crate alloc;

use crate::fs::vfs::Inode;
use crate::syscall::errno::Errno;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Maximum number of open file descriptors per process.
const MAX_FDS: usize = 256;

/// Open file flags.
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const RDONLY: Self = Self {
        read: true,
        write: false,
        append: false,
    };
    pub const WRONLY: Self = Self {
        read: false,
        write: true,
        append: false,
    };
    pub const RDWR: Self = Self {
        read: true,
        write: true,
        append: false,
    };
}

/// An open file descriptor entry.
pub struct FileDescriptor {
    pub inode: Arc<dyn Inode>,
    pub offset: u64,
    pub flags: OpenFlags,
}

/// Per-process file descriptor table.
pub struct FdTable {
    fds: Vec<Option<FileDescriptor>>,
}

impl FdTable {
    pub fn new() -> Self {
        let mut fds = Vec::with_capacity(MAX_FDS);
        fds.resize_with(MAX_FDS, || None);
        Self { fds }
    }

    /// Allocate the lowest available FD and assign the given file.
    pub fn alloc(&mut self, inode: Arc<dyn Inode>, flags: OpenFlags) -> Result<usize, Errno> {
        for (i, slot) in self.fds.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(FileDescriptor {
                    inode,
                    offset: 0,
                    flags,
                });
                return Ok(i);
            }
        }
        Err(Errno::EMFILE)
    }

    /// Get a reference to an open file descriptor.
    pub fn get(&self, fd: usize) -> Result<&FileDescriptor, Errno> {
        self.fds
            .get(fd)
            .and_then(|slot| slot.as_ref())
            .ok_or(Errno::EBADF)
    }

    /// Get a mutable reference to an open file descriptor.
    pub fn get_mut(&mut self, fd: usize) -> Result<&mut FileDescriptor, Errno> {
        self.fds
            .get_mut(fd)
            .and_then(|slot| slot.as_mut())
            .ok_or(Errno::EBADF)
    }

    /// Close a file descriptor.
    pub fn close(&mut self, fd: usize) -> Result<(), Errno> {
        let slot = self.fds.get_mut(fd).ok_or(Errno::EBADF)?;
        if slot.is_none() {
            return Err(Errno::EBADF);
        }
        *slot = None;
        Ok(())
    }

    /// Duplicate a file descriptor to the lowest available slot.
    pub fn dup(&mut self, old_fd: usize) -> Result<usize, Errno> {
        let entry = self.fds.get(old_fd).ok_or(Errno::EBADF)?;
        let entry = entry.as_ref().ok_or(Errno::EBADF)?;
        let inode = entry.inode.clone();
        let flags = entry.flags;
        let offset = entry.offset;

        for (i, slot) in self.fds.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(FileDescriptor {
                    inode,
                    offset,
                    flags,
                });
                return Ok(i);
            }
        }
        Err(Errno::EMFILE)
    }

    /// Duplicate a file descriptor to a specific slot.
    pub fn dup2(&mut self, old_fd: usize, new_fd: usize) -> Result<usize, Errno> {
        if new_fd >= MAX_FDS {
            return Err(Errno::EBADF);
        }
        let entry = self.fds.get(old_fd).ok_or(Errno::EBADF)?;
        let entry = entry.as_ref().ok_or(Errno::EBADF)?;
        let inode = entry.inode.clone();
        let flags = entry.flags;
        let offset = entry.offset;

        // Close new_fd if open
        self.fds[new_fd] = Some(FileDescriptor {
            inode,
            offset,
            flags,
        });
        Ok(new_fd)
    }
}
