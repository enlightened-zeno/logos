extern crate alloc;

use crate::fs::vfs::{DirEntry, FileSystem, Inode, InodeType, Stat};
use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

static NEXT_INODE: AtomicU64 = AtomicU64::new(1);

fn alloc_inode_number() -> u64 {
    NEXT_INODE.fetch_add(1, Ordering::Relaxed)
}

/// In-memory temporary file system.
pub struct TmpFs {
    root: Arc<TmpDir>,
}

impl TmpFs {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            root: Arc::new(TmpDir {
                ino: alloc_inode_number(),
                entries: SpinLock::new(BTreeMap::new()),
            }),
        })
    }
}

impl FileSystem for TmpFs {
    fn name(&self) -> &str {
        "tmpfs"
    }

    fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }
}

/// A tmpfs directory.
struct TmpDir {
    ino: u64,
    entries: SpinLock<BTreeMap<String, Arc<dyn Inode>>>,
}

impl Inode for TmpDir {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(Stat {
            inode: self.ino,
            inode_type: InodeType::Directory,
            size: 0,
            nlink: 2,
            uid: 0,
            gid: 0,
            mode: 0o755,
            dev: 0,
            rdev: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn inode_type(&self) -> InodeType {
        InodeType::Directory
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, Errno> {
        self.entries.lock().get(name).cloned().ok_or(Errno::ENOENT)
    }

    fn create(
        &self,
        name: &str,
        inode_type: InodeType,
        mode: u32,
    ) -> Result<Arc<dyn Inode>, Errno> {
        let mut entries = self.entries.lock();
        if entries.contains_key(name) {
            return Err(Errno::EEXIST);
        }

        let inode: Arc<dyn Inode> = match inode_type {
            InodeType::File => Arc::new(TmpFile {
                ino: alloc_inode_number(),
                data: SpinLock::new(Vec::new()),
                mode,
            }),
            InodeType::Directory => Arc::new(TmpDir {
                ino: alloc_inode_number(),
                entries: SpinLock::new(BTreeMap::new()),
            }),
            _ => return Err(Errno::EINVAL),
        };

        entries.insert(String::from(name), inode.clone());
        Ok(inode)
    }

    fn unlink(&self, name: &str) -> Result<(), Errno> {
        let mut entries = self.entries.lock();
        entries.remove(name).ok_or(Errno::ENOENT)?;
        Ok(())
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        let entries = self.entries.lock();
        let mut result = Vec::with_capacity(entries.len() + 2);

        result.push(DirEntry {
            name: String::from("."),
            inode: self.ino,
            inode_type: InodeType::Directory,
        });
        result.push(DirEntry {
            name: String::from(".."),
            inode: self.ino,
            inode_type: InodeType::Directory,
        });

        for (name, inode) in entries.iter() {
            result.push(DirEntry {
                name: name.clone(),
                inode: inode.stat().map(|s| s.inode).unwrap_or(0),
                inode_type: inode.inode_type(),
            });
        }

        Ok(result)
    }
}

/// A tmpfs file (data stored in a Vec).
struct TmpFile {
    ino: u64,
    data: SpinLock<Vec<u8>>,
    mode: u32,
}

impl Inode for TmpFile {
    fn stat(&self) -> Result<Stat, Errno> {
        let data = self.data.lock();
        Ok(Stat {
            inode: self.ino,
            inode_type: InodeType::File,
            size: data.len() as u64,
            nlink: 1,
            uid: 0,
            gid: 0,
            mode: self.mode,
            dev: 0,
            rdev: 0,
            blksize: 4096,
            blocks: (data.len() as u64).div_ceil(512),
        })
    }

    fn inode_type(&self) -> InodeType {
        InodeType::File
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let data = self.data.lock();
        let start = (offset as usize).min(data.len());
        let end = (start + buf.len()).min(data.len());
        let count = end - start;
        buf[..count].copy_from_slice(&data[start..end]);
        Ok(count)
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<usize, Errno> {
        let mut file_data = self.data.lock();
        let offset = offset as usize;

        // Extend file if writing past end
        if offset + data.len() > file_data.len() {
            file_data.resize(offset + data.len(), 0);
        }

        file_data[offset..offset + data.len()].copy_from_slice(data);
        Ok(data.len())
    }

    fn truncate(&self, size: u64) -> Result<(), Errno> {
        let mut data = self.data.lock();
        data.resize(size as usize, 0);
        Ok(())
    }
}
