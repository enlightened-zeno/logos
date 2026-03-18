/// ext2 on-disk superblock (at byte offset 1024).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_frag_size: u32,
    pub blocks_per_group: u32,
    pub frags_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    // Rev 1+ fields
    pub first_ino: u32,
    pub inode_size: u16,
    pub block_group_nr: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 16],
    pub last_mounted: [u8; 64],
    pub algo_bitmap: u32,
    // Padding to 1024 bytes total
    pub _padding: [u8; 820],
}

pub const EXT2_MAGIC: u16 = 0xEF53;

impl Superblock {
    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size
    }

    pub fn block_group_count(&self) -> u32 {
        self.blocks_count.div_ceil(self.blocks_per_group)
    }

    pub fn inode_size(&self) -> u32 {
        if self.rev_level >= 1 {
            self.inode_size as u32
        } else {
            128
        }
    }
}

/// Block Group Descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BlockGroupDesc {
    pub block_bitmap: u32,
    pub inode_bitmap: u32,
    pub inode_table: u32,
    pub free_blocks_count: u16,
    pub free_inodes_count: u16,
    pub used_dirs_count: u16,
    pub pad: u16,
    pub reserved: [u8; 12],
}

/// On-disk inode.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DiskInode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32, // In 512-byte units
    pub flags: u32,
    pub osd1: u32,
    pub block: [u32; 15], // 0-11: direct, 12: indirect, 13: double indirect, 14: triple indirect
    pub generation: u32,
    pub file_acl: u32,
    pub dir_acl: u32, // size_high for regular files in rev 1
    pub faddr: u32,
    pub osd2: [u8; 12],
}

/// File type bits in inode mode.
pub const S_IFMT: u16 = 0xF000;
pub const S_IFREG: u16 = 0x8000;
pub const S_IFDIR: u16 = 0x4000;
pub const S_IFLNK: u16 = 0xA000;
pub const S_IFCHR: u16 = 0x2000;
pub const S_IFBLK: u16 = 0x6000;

impl DiskInode {
    pub fn file_type(&self) -> u16 {
        self.mode & S_IFMT
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == S_IFDIR
    }

    pub fn is_regular(&self) -> bool {
        self.file_type() == S_IFREG
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type() == S_IFLNK
    }

    /// Get the full 64-bit file size (for rev 1+).
    pub fn size64(&self) -> u64 {
        if self.is_regular() {
            (self.dir_acl as u64) << 32 | self.size as u64
        } else {
            self.size as u64
        }
    }
}

/// On-disk directory entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    // name follows (variable length, up to 255 bytes)
}

/// Directory entry file type values.
pub const FT_UNKNOWN: u8 = 0;
pub const FT_REG_FILE: u8 = 1;
pub const FT_DIR: u8 = 2;
pub const FT_CHRDEV: u8 = 3;
pub const FT_BLKDEV: u8 = 4;
pub const FT_SYMLINK: u8 = 7;

/// Root inode number is always 2 in ext2.
pub const ROOT_INODE: u32 = 2;
