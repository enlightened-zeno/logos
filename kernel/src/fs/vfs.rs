extern crate alloc;

use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Inode type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Pipe,
}

/// File stat information.
#[derive(Debug, Clone)]
pub struct Stat {
    pub inode: u64,
    pub inode_type: InodeType,
    pub size: u64,
    pub nlink: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub dev: u64,
    pub rdev: u64,
    pub blksize: u64,
    pub blocks: u64,
}

/// A directory entry returned by readdir.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode: u64,
    pub inode_type: InodeType,
}

/// Inode trait — represents a file, directory, device, etc.
pub trait Inode: Send + Sync {
    fn stat(&self) -> Result<Stat, Errno>;
    fn inode_type(&self) -> InodeType;

    // File operations
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let _ = (offset, buf);
        Err(Errno::ENOSYS)
    }
    fn write(&self, offset: u64, data: &[u8]) -> Result<usize, Errno> {
        let _ = (offset, data);
        Err(Errno::ENOSYS)
    }
    fn truncate(&self, size: u64) -> Result<(), Errno> {
        let _ = size;
        Err(Errno::ENOSYS)
    }

    // Directory operations
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, Errno> {
        let _ = name;
        Err(Errno::ENOTDIR)
    }
    fn create(
        &self,
        name: &str,
        inode_type: InodeType,
        mode: u32,
    ) -> Result<Arc<dyn Inode>, Errno> {
        let _ = (name, inode_type, mode);
        Err(Errno::ENOSYS)
    }
    fn unlink(&self, name: &str) -> Result<(), Errno> {
        let _ = name;
        Err(Errno::ENOSYS)
    }
    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Err(Errno::ENOTDIR)
    }

    // Symlink
    fn readlink(&self) -> Result<String, Errno> {
        Err(Errno::EINVAL)
    }
}

/// FileSystem trait — mounted file systems implement this.
pub trait FileSystem: Send + Sync {
    fn name(&self) -> &str;
    fn root(&self) -> Arc<dyn Inode>;
}

/// A mount point in the VFS tree.
struct MountPoint {
    path: String,
    fs: Arc<dyn FileSystem>,
}

/// Global VFS state.
pub struct Vfs {
    mounts: Vec<MountPoint>,
}

static VFS: SpinLock<Option<Vfs>> = SpinLock::new(None);

impl Vfs {
    /// Initialize the VFS.
    pub fn init() {
        *VFS.lock() = Some(Vfs { mounts: Vec::new() });
    }

    /// Mount a file system at the given path.
    pub fn mount(path: &str, fs: Arc<dyn FileSystem>) {
        let name = String::from(fs.name());
        let mut guard = VFS.lock();
        let vfs = guard.as_mut().expect("VFS not initialized");
        vfs.mounts.push(MountPoint {
            path: String::from(path),
            fs,
        });
        crate::serial_println!("VFS: mounted {} at {}", name, path);
    }

    /// Resolve a path to an inode, following mount points.
    pub fn resolve(path: &str) -> Result<Arc<dyn Inode>, Errno> {
        let guard = VFS.lock();
        let vfs = guard.as_ref().expect("VFS not initialized");

        // Find the longest matching mount point
        let mut best_mount: Option<&MountPoint> = None;
        let mut best_len = 0;

        for mount in &vfs.mounts {
            if path.starts_with(&mount.path)
                && mount.path.len() >= best_len
                && (path.len() == mount.path.len()
                    || path.as_bytes().get(mount.path.len()) == Some(&b'/'))
            {
                best_mount = Some(mount);
                best_len = mount.path.len();
            }
        }

        let mount = best_mount.ok_or(Errno::ENOENT)?;
        let remaining = &path[best_len..];
        let root = mount.fs.root();

        if remaining.is_empty() || remaining == "/" {
            return Ok(root);
        }

        // Walk the path components
        let mut current = root;
        for component in remaining.split('/').filter(|c| !c.is_empty()) {
            if component == "." {
                continue;
            }
            // ".." handling would need parent tracking — skip for now
            current = current.lookup(component)?;

            // Check for symlinks (with depth limit)
            if current.inode_type() == InodeType::Symlink {
                // For now, don't follow symlinks automatically
                // This will be enhanced later
            }
        }

        Ok(current)
    }

    /// List all mount points.
    pub fn mounts() -> Vec<(String, String)> {
        let guard = VFS.lock();
        let vfs = guard.as_ref().expect("VFS not initialized");
        vfs.mounts
            .iter()
            .map(|m| (m.path.clone(), String::from(m.fs.name())))
            .collect()
    }
}
