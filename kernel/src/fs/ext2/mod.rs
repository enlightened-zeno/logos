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

    /// Write an inode back to disk.
    fn write_inode(&self, ino: u32, inode: &DiskInode) -> Result<(), Errno> {
        let group = ((ino - 1) / self.sb.inodes_per_group) as usize;
        let index = ((ino - 1) % self.sb.inodes_per_group) as usize;
        let inode_size = self.sb.inode_size() as usize;

        let bg = &self.bgdt[group];
        let offset =
            bg.inode_table as u64 * self.block_size as u64 + index as u64 * inode_size as u64;

        // SAFETY: DiskInode is repr(C) and we write exactly its size.
        let bytes = unsafe {
            core::slice::from_raw_parts(inode as *const DiskInode as *const u8, inode_size)
        };
        write_block_bytes(self.write_block, offset, bytes).map_err(|_| Errno::EIO)
    }

    /// Write a data block to disk.
    fn write_data_block(&self, block_num: u32, data: &[u8]) -> Result<(), Errno> {
        let offset = block_num as u64 * self.block_size as u64;
        write_block_bytes(self.write_block, offset, data).map_err(|_| Errno::EIO)
    }

    /// Allocate a free block from the given block group.
    fn alloc_block(&self, preferred_group: usize) -> Result<u32, Errno> {
        let mut inner = self.inner.lock();
        let bg_count = inner.sb.block_group_count() as usize;

        for offset in 0..bg_count {
            let group = (preferred_group + offset) % bg_count;
            let bg = &inner.bgdt[group];

            if bg.free_blocks_count == 0 {
                continue;
            }

            // Read block bitmap
            let mut bitmap = vec![0u8; self.block_size as usize];
            if read_block_bytes(
                self.read_block,
                bg.block_bitmap as u64 * self.block_size as u64,
                &mut bitmap,
            )
            .is_err()
            {
                continue;
            }

            // Find a free bit
            let blocks_in_group = inner
                .sb
                .blocks_per_group
                .min(inner.sb.blocks_count - group as u32 * inner.sb.blocks_per_group);
            for bit in 0..blocks_in_group {
                let byte_idx = bit as usize / 8;
                let bit_idx = bit as usize % 8;
                if bitmap[byte_idx] & (1 << bit_idx) == 0 {
                    // Mark as allocated
                    bitmap[byte_idx] |= 1 << bit_idx;
                    let _ = write_block_bytes(
                        self.write_block,
                        bg.block_bitmap as u64 * self.block_size as u64,
                        &bitmap,
                    );

                    // Update counters
                    inner.bgdt[group].free_blocks_count -= 1;
                    inner.sb.free_blocks_count -= 1;

                    let block_num =
                        group as u32 * inner.sb.blocks_per_group + bit + inner.sb.first_data_block;
                    return Ok(block_num);
                }
            }
        }

        Err(Errno::ENOSPC)
    }

    /// Free a block back to its block group.
    fn free_block(&self, block_num: u32) -> Result<(), Errno> {
        let mut inner = self.inner.lock();
        let group = ((block_num - inner.sb.first_data_block) / inner.sb.blocks_per_group) as usize;
        let bit = ((block_num - inner.sb.first_data_block) % inner.sb.blocks_per_group) as usize;

        let bg = &inner.bgdt[group];
        let mut bitmap = vec![0u8; self.block_size as usize];
        read_block_bytes(
            self.read_block,
            bg.block_bitmap as u64 * self.block_size as u64,
            &mut bitmap,
        )
        .map_err(|_| Errno::EIO)?;

        bitmap[bit / 8] &= !(1 << (bit % 8));
        write_block_bytes(
            self.write_block,
            bg.block_bitmap as u64 * self.block_size as u64,
            &bitmap,
        )
        .map_err(|_| Errno::EIO)?;

        inner.bgdt[group].free_blocks_count += 1;
        inner.sb.free_blocks_count += 1;
        Ok(())
    }

    /// Allocate a free inode from the given block group.
    fn alloc_inode(&self, preferred_group: usize) -> Result<u32, Errno> {
        let mut inner = self.inner.lock();
        let bg_count = inner.sb.block_group_count() as usize;

        for offset in 0..bg_count {
            let group = (preferred_group + offset) % bg_count;
            let bg = &inner.bgdt[group];

            if bg.free_inodes_count == 0 {
                continue;
            }

            let mut bitmap = vec![0u8; self.block_size as usize];
            if read_block_bytes(
                self.read_block,
                bg.inode_bitmap as u64 * self.block_size as u64,
                &mut bitmap,
            )
            .is_err()
            {
                continue;
            }

            for bit in 0..inner.sb.inodes_per_group {
                let byte_idx = bit as usize / 8;
                let bit_idx = bit as usize % 8;
                if bitmap[byte_idx] & (1 << bit_idx) == 0 {
                    bitmap[byte_idx] |= 1 << bit_idx;
                    let _ = write_block_bytes(
                        self.write_block,
                        bg.inode_bitmap as u64 * self.block_size as u64,
                        &bitmap,
                    );

                    inner.bgdt[group].free_inodes_count -= 1;
                    inner.sb.free_inodes_count -= 1;

                    let ino = group as u32 * inner.sb.inodes_per_group + bit + 1;
                    return Ok(ino);
                }
            }
        }

        Err(Errno::ENOSPC)
    }

    /// Write file data to an inode, allocating blocks as needed.
    fn write_file_data(
        &self,
        inode: &mut DiskInode,
        offset: u64,
        data: &[u8],
    ) -> Result<usize, Errno> {
        let mut bytes_written = 0;
        let mut file_offset = offset;
        let bs = self.block_size;

        while bytes_written < data.len() {
            let logical_block = (file_offset / bs as u64) as u32;
            let block_offset = (file_offset % bs as u64) as usize;
            let chunk = (bs as usize - block_offset).min(data.len() - bytes_written);

            // Ensure we have a block allocated for this position
            let mut disk_block = self.resolve_block(inode, logical_block).unwrap_or(0);
            if disk_block == 0 {
                // Allocate a new block
                disk_block = self.alloc_block(0)?;
                // Store in inode (only direct blocks for now)
                if (logical_block as usize) < 12 {
                    inode.block[logical_block as usize] = disk_block;
                } else {
                    return Err(Errno::EFBIG);
                }
                inode.blocks += bs / 512;
            }

            // Read-modify-write if partial block
            let mut block_buf = vec![0u8; bs as usize];
            if block_offset > 0 || chunk < bs as usize {
                self.read_data_block(disk_block, &mut block_buf)?;
            }
            block_buf[block_offset..block_offset + chunk]
                .copy_from_slice(&data[bytes_written..bytes_written + chunk]);
            self.write_data_block(disk_block, &block_buf)?;

            bytes_written += chunk;
            file_offset += chunk as u64;
        }

        // Update size if we extended the file
        let new_size = offset + bytes_written as u64;
        if new_size > inode.size64() {
            inode.size = new_size as u32;
            if inode.is_regular() {
                inode.dir_acl = (new_size >> 32) as u32;
            }
        }

        Ok(bytes_written)
    }

    /// Add a directory entry to a directory inode.
    fn add_dir_entry(
        &self,
        dir_ino: u32,
        dir_inode: &mut DiskInode,
        name: &str,
        child_ino: u32,
        file_type: u8,
    ) -> Result<(), Errno> {
        let name_bytes = name.as_bytes();
        let needed = 8 + name_bytes.len();
        let needed_aligned = needed.next_multiple_of(4);

        let size = dir_inode.size64() as usize;
        let mut data = vec![0u8; size + self.block_size as usize]; // Extra space for growth
        if size > 0 {
            self.read_file_data(dir_inode, 0, &mut data[..size])?;
        }

        // Try to find space in existing entries
        let mut pos = 0;
        while pos + 8 <= size {
            // SAFETY: pos is within bounds.
            let de: DirEntry = unsafe { core::ptr::read(data[pos..].as_ptr() as *const DirEntry) };
            if de.rec_len == 0 {
                break;
            }

            let actual_len = if de.inode != 0 {
                (8 + de.name_len as usize).next_multiple_of(4)
            } else {
                0
            };

            let free_space = de.rec_len as usize - actual_len;
            if free_space >= needed_aligned {
                // Split this entry
                let old_rec_len = de.rec_len;

                // Shrink existing entry
                // SAFETY: Writing within allocated buffer.
                unsafe {
                    let de_ptr = data[pos..].as_mut_ptr() as *mut DirEntry;
                    (*de_ptr).rec_len = actual_len as u16;
                }

                // Write new entry after it
                let new_pos = pos + actual_len;
                let new_de = DirEntry {
                    inode: child_ino,
                    rec_len: (old_rec_len as usize - actual_len) as u16,
                    name_len: name_bytes.len() as u8,
                    file_type,
                };
                // SAFETY: new_pos + 8 is within buffer bounds.
                unsafe {
                    core::ptr::write(data[new_pos..].as_mut_ptr() as *mut DirEntry, new_de);
                }
                data[new_pos + 8..new_pos + 8 + name_bytes.len()].copy_from_slice(name_bytes);

                // Write back the block containing the directory data
                self.write_file_data(dir_inode, 0, &data[..size])?;
                self.write_inode(dir_ino, dir_inode)?;
                return Ok(());
            }

            pos += de.rec_len as usize;
        }

        // No space found — append at end (extend directory)
        let new_de = DirEntry {
            inode: child_ino,
            rec_len: self.block_size as u16, // Takes the rest of the new block
            name_len: name_bytes.len() as u8,
            file_type,
        };
        let mut new_entry = vec![0u8; self.block_size as usize];
        // SAFETY: Writing into freshly allocated buffer.
        unsafe {
            core::ptr::write(new_entry.as_mut_ptr() as *mut DirEntry, new_de);
        }
        new_entry[8..8 + name_bytes.len()].copy_from_slice(name_bytes);

        self.write_file_data(dir_inode, size as u64, &new_entry)?;
        self.write_inode(dir_ino, dir_inode)?;
        Ok(())
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

    fn write(&self, offset: u64, data: &[u8]) -> Result<usize, Errno> {
        let mut di = self.disk_inode()?;
        if !di.is_regular() {
            return Err(Errno::EINVAL);
        }
        let written = self.fs().write_file_data(&mut di, offset, data)?;
        self.fs().write_inode(self.ino, &di)?;
        Ok(written)
    }

    fn create(
        &self,
        name: &str,
        inode_type: InodeType,
        mode: u32,
    ) -> Result<Arc<dyn Inode>, Errno> {
        let mut dir_inode = self.disk_inode()?;
        if !dir_inode.is_dir() {
            return Err(Errno::ENOTDIR);
        }

        // Check if name already exists
        let entries = self.fs().read_dir_entries(&dir_inode)?;
        if entries.iter().any(|(n, _, _)| n == name) {
            return Err(Errno::EEXIST);
        }

        let group = ((self.ino - 1) / self.fs().sb.inodes_per_group) as usize;
        let new_ino = self.fs().alloc_inode(group)?;

        let (file_mode, ft) = match inode_type {
            InodeType::File => (S_IFREG | (mode as u16 & 0o777), FT_REG_FILE),
            InodeType::Directory => (S_IFDIR | (mode as u16 & 0o777), FT_DIR),
            _ => return Err(Errno::EINVAL),
        };

        let mut new_inode = DiskInode {
            mode: file_mode,
            uid: 0,
            size: 0,
            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,
            gid: 0,
            links_count: 1,
            blocks: 0,
            flags: 0,
            osd1: 0,
            block: [0; 15],
            generation: 0,
            file_acl: 0,
            dir_acl: 0,
            faddr: 0,
            osd2: [0; 12],
        };

        if inode_type == InodeType::Directory {
            new_inode.links_count = 2; // . and parent's link
            dir_inode.links_count += 1; // .. in new dir points back

            // Create . and .. entries in the new directory
            let block = self.fs().alloc_block(group)?;
            new_inode.block[0] = block;
            new_inode.blocks = self.fs().block_size / 512;
            new_inode.size = self.fs().block_size;

            let mut dir_data = vec![0u8; self.fs().block_size as usize];
            // . entry
            let dot = DirEntry {
                inode: new_ino,
                rec_len: 12,
                name_len: 1,
                file_type: FT_DIR,
            };
            // SAFETY: Writing into freshly allocated buffer.
            unsafe { core::ptr::write(dir_data.as_mut_ptr() as *mut DirEntry, dot) };
            dir_data[8] = b'.';

            // .. entry (takes rest of block)
            let dotdot = DirEntry {
                inode: self.ino,
                rec_len: (self.fs().block_size as u16) - 12,
                name_len: 2,
                file_type: FT_DIR,
            };
            // SAFETY: Writing at offset 12 in the buffer.
            unsafe {
                core::ptr::write(dir_data[12..].as_mut_ptr() as *mut DirEntry, dotdot);
            }
            dir_data[20] = b'.';
            dir_data[21] = b'.';

            self.fs().write_data_block(block, &dir_data)?;
        }

        self.fs().write_inode(new_ino, &new_inode)?;
        self.fs()
            .add_dir_entry(self.ino, &mut dir_inode, name, new_ino, ft)?;

        Ok(Arc::new(Ext2Inode {
            fs: self.fs,
            ino: new_ino,
        }))
    }

    fn unlink(&self, name: &str) -> Result<(), Errno> {
        let mut dir_inode = self.disk_inode()?;
        if !dir_inode.is_dir() {
            return Err(Errno::ENOTDIR);
        }

        // Find the entry
        let size = dir_inode.size64() as usize;
        let mut data = vec![0u8; size];
        self.fs().read_file_data(&dir_inode, 0, &mut data)?;

        let mut pos = 0;
        let mut prev_pos: Option<usize> = None;
        let mut found_ino = 0u32;

        while pos + 8 <= size {
            // SAFETY: pos is within bounds.
            let de: DirEntry = unsafe { core::ptr::read(data[pos..].as_ptr() as *const DirEntry) };
            if de.rec_len == 0 {
                break;
            }

            if de.inode != 0 && de.name_len > 0 {
                let name_start = pos + 8;
                let name_end = name_start + de.name_len as usize;
                if name_end <= size {
                    let entry_name =
                        core::str::from_utf8(&data[name_start..name_end]).unwrap_or("");
                    if entry_name == name {
                        found_ino = de.inode;

                        // Mark entry as deleted by setting inode to 0
                        // SAFETY: Writing within bounds.
                        unsafe {
                            let de_ptr = data[pos..].as_mut_ptr() as *mut DirEntry;
                            (*de_ptr).inode = 0;
                        }

                        // If there's a previous entry, merge the space
                        if let Some(pp) = prev_pos {
                            // SAFETY: prev_pos is a valid entry position.
                            unsafe {
                                let prev_ptr = data[pp..].as_mut_ptr() as *mut DirEntry;
                                (*prev_ptr).rec_len += de.rec_len;
                            }
                        }
                        break;
                    }
                }
            }

            prev_pos = Some(pos);
            pos += de.rec_len as usize;
        }

        if found_ino == 0 {
            return Err(Errno::ENOENT);
        }

        // Write back modified directory data
        self.fs()
            .write_file_data(&mut dir_inode, 0, &data[..size])?;
        self.fs().write_inode(self.ino, &dir_inode)?;

        // Decrement link count on the removed inode
        let mut child = self.fs().read_inode(found_ino)?;
        if child.links_count > 0 {
            child.links_count -= 1;
        }
        self.fs().write_inode(found_ino, &child)?;

        Ok(())
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

/// Write bytes to the block device at an arbitrary byte offset.
fn write_block_bytes(
    write_fn: BlockWriteFn,
    byte_offset: u64,
    buf: &[u8],
) -> Result<(), &'static str> {
    use crate::drivers::virtio::block::SECTOR_SIZE;

    let start_sector = byte_offset / SECTOR_SIZE;
    let offset_in_sector = (byte_offset % SECTOR_SIZE) as usize;

    if offset_in_sector == 0 && buf.len().is_multiple_of(SECTOR_SIZE as usize) {
        // Aligned write — direct
        return write_fn(start_sector, buf);
    }

    // Unaligned — read-modify-write
    let total_bytes = offset_in_sector + buf.len();
    let sectors_needed = (total_bytes as u64).div_ceil(SECTOR_SIZE);
    let mut sector_buf = vec![0u8; (sectors_needed * SECTOR_SIZE) as usize];

    // Zero-fill non-overwritten parts (RMW would need the read_fn too).
    sector_buf[offset_in_sector..offset_in_sector + buf.len()].copy_from_slice(buf);
    write_fn(start_sector, &sector_buf)
}
