extern crate alloc;

use crate::fs::vfs::{DirEntry, FileSystem, Inode, InodeType, Stat};
use crate::syscall::errno::Errno;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Process information file system mounted at /proc.
pub struct ProcFs;

impl ProcFs {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl FileSystem for ProcFs {
    fn name(&self) -> &str {
        "procfs"
    }
    fn root(&self) -> Arc<dyn Inode> {
        Arc::new(ProcRoot)
    }
}

struct ProcRoot;

impl Inode for ProcRoot {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(proc_dir_stat(1))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::Directory
    }
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, Errno> {
        match name {
            "uptime" => Ok(Arc::new(ProcUptime)),
            "meminfo" => Ok(Arc::new(ProcMeminfo)),
            "version" => Ok(Arc::new(ProcVersion)),
            "mounts" => Ok(Arc::new(ProcMounts)),
            _ => Err(Errno::ENOENT),
        }
    }
    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Ok(alloc::vec![
            DirEntry {
                name: String::from("."),
                inode: 1,
                inode_type: InodeType::Directory
            },
            DirEntry {
                name: String::from(".."),
                inode: 1,
                inode_type: InodeType::Directory
            },
            DirEntry {
                name: String::from("uptime"),
                inode: 2,
                inode_type: InodeType::File
            },
            DirEntry {
                name: String::from("meminfo"),
                inode: 3,
                inode_type: InodeType::File
            },
            DirEntry {
                name: String::from("version"),
                inode: 4,
                inode_type: InodeType::File
            },
            DirEntry {
                name: String::from("mounts"),
                inode: 5,
                inode_type: InodeType::File
            },
        ])
    }
}

/// /proc/uptime — system uptime in seconds
struct ProcUptime;
impl Inode for ProcUptime {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(proc_file_stat(2))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::File
    }
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let ticks = crate::arch::x86_64::apic::ticks();
        let secs = ticks / 1000;
        let frac = (ticks % 1000) / 10;
        let content = format!("{}.{:02} 0.00\n", secs, frac);
        read_generated(&content, offset, buf)
    }
}

/// /proc/meminfo — memory statistics
struct ProcMeminfo;
impl Inode for ProcMeminfo {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(proc_file_stat(3))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::File
    }
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let pmm = crate::memory::pmm::Pmm::get();
        let total_kb = pmm.total_frames() * 4;
        let free_kb = pmm.free_frames() * 4;
        let used_kb = total_kb - free_kb;
        let content = format!(
            "MemTotal:    {:>8} kB\nMemFree:     {:>8} kB\nMemUsed:     {:>8} kB\n",
            total_kb, free_kb, used_kb
        );
        read_generated(&content, offset, buf)
    }
}

/// /proc/version — kernel version string
struct ProcVersion;
impl Inode for ProcVersion {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(proc_file_stat(4))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::File
    }
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let content = "LogOS version 0.1.0 (x86_64)\n";
        read_generated(content, offset, buf)
    }
}

/// /proc/mounts — mounted filesystems
struct ProcMounts;
impl Inode for ProcMounts {
    fn stat(&self) -> Result<Stat, Errno> {
        Ok(proc_file_stat(5))
    }
    fn inode_type(&self) -> InodeType {
        InodeType::File
    }
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let mounts = crate::fs::vfs::Vfs::mounts();
        let mut content = String::new();
        for (path, fsname) in &mounts {
            content.push_str(&format!("{} {} {} rw 0 0\n", fsname, path, fsname));
        }
        read_generated(&content, offset, buf)
    }
}

fn read_generated(content: &str, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
    let bytes = content.as_bytes();
    let start = (offset as usize).min(bytes.len());
    let end = (start + buf.len()).min(bytes.len());
    let count = end - start;
    buf[..count].copy_from_slice(&bytes[start..end]);
    Ok(count)
}

fn proc_dir_stat(ino: u64) -> Stat {
    Stat {
        inode: ino,
        inode_type: InodeType::Directory,
        size: 0,
        nlink: 2,
        uid: 0,
        gid: 0,
        mode: 0o555,
        dev: 0,
        rdev: 0,
        blksize: 4096,
        blocks: 0,
    }
}

fn proc_file_stat(ino: u64) -> Stat {
    Stat {
        inode: ino,
        inode_type: InodeType::File,
        size: 0,
        nlink: 1,
        uid: 0,
        gid: 0,
        mode: 0o444,
        dev: 0,
        rdev: 0,
        blksize: 4096,
        blocks: 0,
    }
}
