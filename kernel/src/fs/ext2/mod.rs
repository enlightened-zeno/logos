pub mod ondisk;

extern crate alloc;

use crate::fs::vfs::{DirEntry as VfsDirEntry, FileSystem, Inode, InodeType, Stat};
use crate::sync::SpinLock;
use crate::syscall::errno::Errno;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use ondisk::*;

/// Block read/write function type.
type BlockReadFn = fn(block: u64, buf: &mut [u8]) -> Result<(), &'static str>;
type BlockWriteFn = fn(block: u64, buf: &[u8]) -> Result<(), &'static str>;

/// ext2 filesystem instance.
pub struct Ext2Fs {
    sb: Superblock,
    block_size: u32,
    bgdt: Vec<BlockGroupDesc>,
    read_block: BlockReadFn,
    write_block: BlockWriteFn,
    inner: SpinLock<Ext2Inner>,
}

struct Ext2Inner {
    sb: Superblock,
    bgdt: Vec<BlockGroupDesc>,
}

impl Ext2Fs {
    /// Mount an ext2 filesystem from a block device.
    pub fn mount(
        read_block: BlockReadFn,
        write_block: BlockWriteFn,
    ) -> Result<Arc<Self>, &'static str> {
        // Read the superblock (at byte offset 1024 = block 1 for 1K blocks, or
        // within block 0 for larger blocks)
        let mut sb_buf = vec![0u8; 1024];
        // Superblock is always at byte 1024, which is sector 2
        read_block_bytes(read_block, 1024, &mut sb_buf)?;

        // SAFETY: We read exactly 1024 bytes and Superblock is repr(C) with size 1024.
        let sb: Superblock = unsafe { core::ptr::read(sb_buf.as_ptr() as *const Superblock) };

        if sb.magic != EXT2_MAGIC {
            return Err("ext2: bad magic number");
        }

        let block_size = sb.block_size();
        let bg_count = sb.block_group_count();

        // Read block group descriptor table (starts at block 2 for 1K blocks,
        // block 1 for larger blocks)
        let bgdt_offset = if block_size == 1024 {
            2048
        } else {
            block_size as u64
        };
        let bgdt_size = bg_count as usize * core::mem::size_of::<BlockGroupDesc>();
        let mut bgdt_buf = vec![0u8; bgdt_size];
        read_block_bytes(read_block, bgdt_offset, &mut bgdt_buf)?;

        let mut bgdt = Vec::with_capacity(bg_count as usize);
        for i in 0..bg_count as usize {
            let offset = i * core::mem::size_of::<BlockGroupDesc>();
            let ptr = bgdt_buf[offset..].as_ptr() as *const BlockGroupDesc;
            // SAFETY: Buffer was read from disk and contains valid block group descriptors.
            let desc: BlockGroupDesc = unsafe { core::ptr::read(ptr) };
            bgdt.push(desc);
        }

        crate::serial_println!(
            "ext2: {} blocks, {} inodes, block_size={}, {} groups",
            sb.blocks_count,
            sb.inodes_count,
            block_size,
            bg_count
        );

        let inner = Ext2Inner {
            sb,
            bgdt: bgdt.clone(),
        };

        Ok(Arc::new(Self {
            sb,
            block_size,
            bgdt,
            read_block,
            write_block,
            inner: SpinLock::new(inner),
        }))
    }

    /// Read an inode from disk.
    fn read_inode(&self, ino: u32) -> Result<DiskInode, Errno> {
        let group = ((ino - 1) / self.sb.inodes_per_group) as usize;
        let index = ((ino - 1) % self.sb.inodes_per_group) as usize;
        let inode_size = self.sb.inode_size() as usize;

        let bg = &self.bgdt[group];
        let offset =
            bg.inode_table as u64 * self.block_size as u64 + index as u64 * inode_size as u64;

        let mut buf = vec![0u8; inode_size];
        read_block_bytes(self.read_block, offset, &mut buf).map_err(|_| Errno::EIO)?;

        // SAFETY: buf contains a valid on-disk inode structure.
        Ok(unsafe { core::ptr::read(buf.as_ptr() as *const DiskInode) })
    }

    /// Read a data block from the filesystem.
    fn read_data_block(&self, block_num: u32, buf: &mut [u8]) -> Result<(), Errno> {
        let offset = block_num as u64 * self.block_size as u64;
        read_block_bytes(self.read_block, offset, buf).map_err(|_| Errno::EIO)
    }

    /// Resolve the disk block number for a given logical block of an inode.
    fn resolve_block(&self, inode: &DiskInode, logical_block: u32) -> Result<u32, Errno> {
        let ptrs_per_block = self.block_size / 4;

        if logical_block < 12 {
            // Direct blocks
            return Ok(inode.block[logical_block as usize]);
        }

        let logical_block = logical_block - 12;
        if logical_block < ptrs_per_block {
            // Single indirect
            return self.read_indirect(inode.block[12], logical_block);
        }

        let logical_block = logical_block - ptrs_per_block;
        if logical_block < ptrs_per_block * ptrs_per_block {
            // Double indirect
            let idx1 = logical_block / ptrs_per_block;
            let idx2 = logical_block % ptrs_per_block;
            let indirect1 = self.read_indirect(inode.block[13], idx1)?;
            return self.read_indirect(indirect1, idx2);
        }

        Err(Errno::EFBIG)
    }

    fn read_indirect(&self, block: u32, index: u32) -> Result<u32, Errno> {
        if block == 0 {
            return Ok(0);
        }
        let mut buf = vec![0u8; self.block_size as usize];
        self.read_data_block(block, &mut buf)?;
        let ptrs = unsafe {
            core::slice::from_raw_parts(buf.as_ptr() as *const u32, self.block_size as usize / 4)
        };
        Ok(ptrs[index as usize])
    }

    /// Read file data from an inode.
    fn read_file_data(
        &self,
        inode: &DiskInode,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<usize, Errno> {
        let file_size = inode.size64();
        if offset >= file_size {
            return Ok(0);
        }

        let to_read = buf.len().min((file_size - offset) as usize);
        let mut bytes_read = 0;
        let mut file_offset = offset;

        let mut block_buf = vec![0u8; self.block_size as usize];

        while bytes_read < to_read {
            let logical_block = (file_offset / self.block_size as u64) as u32;
            let block_offset = (file_offset % self.block_size as u64) as usize;
            let chunk = (self.block_size as usize - block_offset).min(to_read - bytes_read);

            let disk_block = self.resolve_block(inode, logical_block)?;
            if disk_block == 0 {
                // Sparse block — fill with zeros
                buf[bytes_read..bytes_read + chunk].fill(0);
            } else {
                self.read_data_block(disk_block, &mut block_buf)?;
                buf[bytes_read..bytes_read + chunk]
                    .copy_from_slice(&block_buf[block_offset..block_offset + chunk]);
            }

            bytes_read += chunk;
            file_offset += chunk as u64;
        }

        Ok(bytes_read)
    }

    /// Read directory entries from an inode.
    fn read_dir_entries(&self, inode: &DiskInode) -> Result<Vec<(String, u32, u8)>, Errno> {
        let size = inode.size64() as usize;
        let mut data = vec![0u8; size];
        self.read_file_data(inode, 0, &mut data)?;

        let mut entries = Vec::new();
        let mut pos = 0;

        while pos + 8 <= size {
            // SAFETY: pos is within bounds and we check rec_len.
            let de: DirEntry = unsafe { core::ptr::read(data[pos..].as_ptr() as *const DirEntry) };

            if de.rec_len == 0 {
                break;
            }
            if de.inode != 0 && de.name_len > 0 {
                let name_start = pos + 8;
                let name_end = name_start + de.name_len as usize;
                if name_end <= size {
                    let name = String::from(
                        core::str::from_utf8(&data[name_start..name_end]).unwrap_or("?"),
                    );
                    entries.push((name, de.inode, de.file_type));
                }
            }

            pos += de.rec_len as usize;
        }

        Ok(entries)
    }
}

impl FileSystem for Ext2Fs {
    fn name(&self) -> &str {
        "ext2"
    }

    fn root(&self) -> Arc<dyn Inode> {
        Arc::new(Ext2Inode {
            fs: self as *const Ext2Fs,
            ino: ROOT_INODE,
        })
    }
}

// SAFETY: Ext2Fs is protected by internal SpinLocks and is read-only for most ops.
unsafe impl Send for Ext2Fs {}
unsafe impl Sync for Ext2Fs {}

/// An ext2 inode wrapper for the VFS.
struct Ext2Inode {
    fs: *const Ext2Fs,
    ino: u32,
}

// SAFETY: The Ext2Fs pointer is valid for the lifetime of the filesystem.
unsafe impl Send for Ext2Inode {}
unsafe impl Sync for Ext2Inode {}

impl Ext2Inode {
    fn fs(&self) -> &Ext2Fs {
        // SAFETY: The Ext2Fs lives as long as any Ext2Inode referencing it.
        unsafe { &*self.fs }
    }

    fn disk_inode(&self) -> Result<DiskInode, Errno> {
        self.fs().read_inode(self.ino)
    }
}

impl Inode for Ext2Inode {
    fn stat(&self) -> Result<Stat, Errno> {
        let di = self.disk_inode()?;
        Ok(Stat {
            inode: self.ino as u64,
            inode_type: disk_type_to_vfs(di.file_type()),
            size: di.size64(),
            nlink: di.links_count as u64,
            uid: di.uid as u32,
            gid: di.gid as u32,
            mode: di.mode as u32,
            dev: 0,
            rdev: 0,
            blksize: self.fs().block_size as u64,
            blocks: di.blocks as u64,
        })
    }

    fn inode_type(&self) -> InodeType {
        self.disk_inode()
            .map(|di| disk_type_to_vfs(di.file_type()))
            .unwrap_or(InodeType::File)
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Errno> {
        let di = self.disk_inode()?;
        self.fs().read_file_data(&di, offset, buf)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, Errno> {
        let di = self.disk_inode()?;
        if !di.is_dir() {
            return Err(Errno::ENOTDIR);
        }

        let entries = self.fs().read_dir_entries(&di)?;
        for (entry_name, ino, _ft) in &entries {
            if entry_name == name {
                return Ok(Arc::new(Ext2Inode {
                    fs: self.fs,
                    ino: *ino,
                }));
            }
        }

        Err(Errno::ENOENT)
    }

    fn readdir(&self) -> Result<Vec<VfsDirEntry>, Errno> {
        let di = self.disk_inode()?;
        if !di.is_dir() {
            return Err(Errno::ENOTDIR);
        }

        let entries = self.fs().read_dir_entries(&di)?;
        Ok(entries
            .into_iter()
            .map(|(name, ino, ft)| VfsDirEntry {
                name,
                inode: ino as u64,
                inode_type: ft_to_vfs(ft),
            })
            .collect())
    }

    fn readlink(&self) -> Result<String, Errno> {
        let di = self.disk_inode()?;
        if !di.is_symlink() {
            return Err(Errno::EINVAL);
        }

        let size = di.size64() as usize;
        if size <= 60 {
            let ptr = di.block.as_ptr() as *const u8;
            // SAFETY: Fast symlink stores target inline in the 60-byte block pointer
            // area. Size is verified to be <= 60.
            let bytes = unsafe { core::slice::from_raw_parts(ptr, size) };
            Ok(String::from(
                core::str::from_utf8(bytes).map_err(|_| Errno::EIO)?,
            ))
        } else {
            let mut buf = vec![0u8; size];
            self.fs().read_file_data(&di, 0, &mut buf)?;
            Ok(String::from(
                core::str::from_utf8(&buf).map_err(|_| Errno::EIO)?,
            ))
        }
    }
}

fn disk_type_to_vfs(mode_type: u16) -> InodeType {
    match mode_type {
        S_IFREG => InodeType::File,
        S_IFDIR => InodeType::Directory,
        S_IFLNK => InodeType::Symlink,
        S_IFCHR => InodeType::CharDevice,
        S_IFBLK => InodeType::BlockDevice,
        _ => InodeType::File,
    }
}

fn ft_to_vfs(ft: u8) -> InodeType {
    match ft {
        FT_REG_FILE => InodeType::File,
        FT_DIR => InodeType::Directory,
        FT_SYMLINK => InodeType::Symlink,
        FT_CHRDEV => InodeType::CharDevice,
        FT_BLKDEV => InodeType::BlockDevice,
        _ => InodeType::File,
    }
}

/// Read bytes from the block device at an arbitrary byte offset.
fn read_block_bytes(
    read_fn: BlockReadFn,
    byte_offset: u64,
    buf: &mut [u8],
) -> Result<(), &'static str> {
    use crate::drivers::virtio::block::SECTOR_SIZE;

    let start_sector = byte_offset / SECTOR_SIZE;
    let offset_in_sector = (byte_offset % SECTOR_SIZE) as usize;

    // Read enough sectors to cover the request
    let total_bytes = offset_in_sector + buf.len();
    let sectors_needed = (total_bytes as u64).div_ceil(SECTOR_SIZE);
    let mut sector_buf = vec![0u8; (sectors_needed * SECTOR_SIZE) as usize];

    read_fn(start_sector, &mut sector_buf)?;
    buf.copy_from_slice(&sector_buf[offset_in_sector..offset_in_sector + buf.len()]);
    Ok(())
}
