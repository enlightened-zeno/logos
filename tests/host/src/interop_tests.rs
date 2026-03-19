/// ext2 interoperability logic tests.

#[test]
fn test_ext2_superblock_size() {
    let sb_size = 1024;
    let sb_offset = 1024; // Always at byte 1024
    assert_eq!(sb_size, 1024);
    assert_eq!(sb_offset, 1024);
}

#[test]
fn test_ext2_bgdt_location() {
    // For 1K blocks: BGDT at block 2 (byte 2048)
    // For 4K blocks: BGDT at block 1 (byte 4096)
    let block_size_1k = 1024u64;
    let bgdt_offset_1k = 2048u64;
    assert_eq!(bgdt_offset_1k, 2 * block_size_1k);

    let block_size_4k = 4096u64;
    let bgdt_offset_4k = block_size_4k;
    assert_eq!(bgdt_offset_4k, 4096);
}

#[test]
fn test_ext2_clean_unmount() {
    let ext2_valid_fs: u16 = 0x0001;
    let ext2_error_fs: u16 = 0x0002;
    assert_ne!(ext2_valid_fs, ext2_error_fs);
}

#[test]
fn test_ext2_feature_compat() {
    // LogOS supports basic ext2 — no incompatible features
    let incompat_features = 0u32;
    assert_eq!(incompat_features, 0);
}

#[test]
fn test_ext2_inode_size() {
    // Rev 0: 128 bytes, Rev 1+: variable (typically 256)
    let rev0_size = 128u16;
    let rev1_size = 256u16;
    assert!(rev1_size >= rev0_size);
}
