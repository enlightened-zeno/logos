extern crate alloc;

use crate::fs::vfs::{DirEntry, FileSystem, Inode, InodeType, Stat};
use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Device file system mounted at /dev.
pub struct DevFs {
    root: Arc<DevDir>,
}

impl DevFs {
    pub fn new() -> Arc<Self> {
        let mut entries: BTreeMap<String, Arc<dyn Inode>> = BTreeMap::new();
        entries.insert(String::from("null"), Arc::new(DevNull));
        entries.insert(String::from("zero"), Arc::new(DevZero));
        entries.insert(String::from("random"), Arc::new(DevRandom));
        entries.insert(String::from("urandom"), Arc::new(DevRandom));
        entries.insert(String::from("console"), Arc::new(DevConsole));
        entries.insert(String::from("tty"), Arc::new(DevConsole));

        Arc::new(Self {
            root: Arc::new(DevDir {
                entries: SpinLock::new(entries),
            }),
        })
    }
}

impl FileSystem for DevFs {
    fn name(&self) -> &str {
        "devfs"
    }
    fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }
}

struct DevDir {
    entries: SpinLock<BTreeMap<String, Arc<dyn Inode>>>,
}

impl Inode for DevDir {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(Stat {
            inode: 1,
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
    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        let entries = self.entries.lock();
        let mut result = Vec::new();
        result.push(DirEntry {
            name: String::from("."),
            inode: 1,
            inode_type: InodeType::Directory,
        });
        result.push(DirEntry {
            name: String::from(".."),
            inode: 1,
            inode_type: InodeType::Directory,
        });
        for (name, inode) in entries.iter() {
            result.push(DirEntry {
                name: name.clone(),
                inode: 0,
                inode_type: inode.inode_type(),
            });
        }
        Ok(result)
    }
}

// /dev/null — discards writes, reads return EOF
struct DevNull;
impl Inode for DevNull {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(dev_stat(2, InodeType::CharDevice, 1, 3))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::CharDevice
    }
    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, Errno> {
        Ok(0)
    }
    fn write(&self, _offset: u64, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }
}

// /dev/zero — reads return zeros, discards writes
struct DevZero;
impl Inode for DevZero {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(dev_stat(3, InodeType::CharDevice, 1, 5))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::CharDevice
    }
    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write(&self, _offset: u64, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }
}

// /dev/random, /dev/urandom — reads return random bytes
struct DevRandom;
impl Inode for DevRandom {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(dev_stat(4, InodeType::CharDevice, 1, 8))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::CharDevice
    }
    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        crate::entropy::fill_bytes(buf);
        Ok(buf.len())
    }
    fn write(&self, _offset: u64, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }
}

// /dev/console, /dev/tty — serial console
struct DevConsole;
impl Inode for DevConsole {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(dev_stat(5, InodeType::CharDevice, 5, 1))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::CharDevice
    }
    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, Errno> {
        // No input yet — return 0 (EOF)
        Ok(0)
    }
    fn write(&self, _offset: u64, data: &[u8]) -> Result<usize, Errno> {
        for &byte in data {
            crate::drivers::serial::write_byte(byte);
        }
        Ok(data.len())
    }
}

fn dev_stat(ino: u64, itype: InodeType, major: u64, minor: u64) -> Stat {
    Stat {
        inode: ino,
        inode_type: itype,
        size: 0,
        nlink: 1,
        uid: 0,
        gid: 0,
        mode: 0o666,
        dev: 0,
        rdev: (major << 8) | minor,
        blksize: 4096,
        blocks: 0,
    }
}
