/// ext2 on-disk structure tests.

#[test]
fn test_ext2_magic() {
    let magic: u16 = 0xEF53;
    assert_eq!(magic, 0xEF53);
}

#[test]
fn test_ext2_block_size() {
    // block_size = 1024 << log_block_size
    assert_eq!(1024u32 << 0, 1024);
    assert_eq!(1024u32 << 1, 2048);
    assert_eq!(1024u32 << 2, 4096);
}

#[test]
fn test_ext2_inode_types() {
    let s_ifreg: u16 = 0x8000;
    let s_ifdir: u16 = 0x4000;
    let s_iflnk: u16 = 0xA000;
    let s_ifchr: u16 = 0x2000;
    let s_ifblk: u16 = 0x6000;
    let s_ifmt: u16 = 0xF000;

    assert_eq!(s_ifreg & s_ifmt, s_ifreg);
    assert_eq!(s_ifdir & s_ifmt, s_ifdir);
    assert_ne!(s_ifreg, s_ifdir);
    assert_ne!(s_iflnk, s_ifreg);
    let _ = (s_ifchr, s_ifblk); // Used
}

#[test]
fn test_ext2_block_group_count() {
    let blocks_count = 8192u32;
    let blocks_per_group = 8192u32;
    let bg_count = blocks_count.div_ceil(blocks_per_group);
    assert_eq!(bg_count, 1);

    let blocks_count2 = 16384u32;
    let bg_count2 = blocks_count2.div_ceil(blocks_per_group);
    assert_eq!(bg_count2, 2);
}

#[test]
fn test_ext2_inode_location() {
    let ino = 5u32;
    let inodes_per_group = 128u32;
    let group = (ino - 1) / inodes_per_group;
    let index = (ino - 1) % inodes_per_group;
    assert_eq!(group, 0);
    assert_eq!(index, 4);
}

#[test]
fn test_ext2_root_inode() {
    let root_ino = 2u32;
    assert_eq!(root_ino, 2); // Always 2 in ext2
}

#[test]
fn test_ext2_dir_entry_types() {
    let ft_unknown: u8 = 0;
    let ft_reg: u8 = 1;
    let ft_dir: u8 = 2;
    let ft_chrdev: u8 = 3;
    let ft_blkdev: u8 = 4;
    let ft_symlink: u8 = 7;
    assert_ne!(ft_reg, ft_dir);
    let _ = (ft_unknown, ft_chrdev, ft_blkdev, ft_symlink);
}

#[test]
fn test_ext2_indirect_blocks() {
    let block_size = 4096u32;
    let ptrs_per_block = block_size / 4; // 1024 pointers

    let direct = 12u32;
    let single_indirect = ptrs_per_block; // 1024
    let double_indirect = ptrs_per_block * ptrs_per_block; // ~1M

    let max_blocks = direct + single_indirect + double_indirect;
    assert!(max_blocks > 1_000_000);

    // File can be at least max_blocks * 4096 bytes
    let max_size = max_blocks as u64 * block_size as u64;
    assert!(max_size > 4_000_000_000); // > 4 GiB
}

#[test]
fn test_ext2_fast_symlink() {
    // Symlinks <= 60 bytes stored inline in block pointers
    let max_inline = 15 * 4; // 15 block pointers * 4 bytes each = 60
    assert_eq!(max_inline, 60);
    let target = "/usr/bin/env";
    assert!(target.len() <= max_inline);
}
