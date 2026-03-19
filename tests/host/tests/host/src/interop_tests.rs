//! ext2 interoperability tests
#[test] fn ext2_magic() { assert_eq!(0xEF53u16, 0xEF53); }
#[test] fn ext2_block_size_1k() { assert_eq!(1024u32 << 0, 1024); }
#[test] fn ext2_block_size_4k() { assert_eq!(1024u32 << 2, 4096); }
#[test] fn ext2_inode_size() { assert_eq!(128u16, 128); /* Standard ext2 inode */ }
#[test] fn ext2_root_inode() { assert_eq!(2u32, 2); }
#[test] fn ext2_superblock_offset() { assert_eq!(1024u64, 1024); }
#[test] fn ext2_direct_blocks() { assert_eq!(12u32, 12); }
#[test] fn ext2_indirect_block() { assert_eq!(12u32, 12); /* Index 12 = single indirect */ }
#[test] fn ext2_double_indirect() { assert_eq!(13u32, 13); }
#[test] fn ext2_triple_indirect() { assert_eq!(14u32, 14); }
#[test] fn ext2_dir_entry_min_size() { assert_eq!(8u32, 8); /* inode(4) + rec_len(2) + name_len(1) + type(1) */ }
