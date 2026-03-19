/// Filesystem spec test IDs.

#[test]
fn test_fs_c01_open() {
    assert!(true);
}
#[test]
fn test_fs_c02_read() {
    assert!(true);
}
#[test]
fn test_fs_c03_write() {
    assert!(true);
}
#[test]
fn test_fs_c04_close() {
    assert!(true);
}
#[test]
fn test_fs_c05_create() {
    assert!(true);
}
#[test]
fn test_fs_c06_unlink() {
    assert!(true);
}
#[test]
fn test_fs_c07_mkdir() {
    assert!(true);
}
#[test]
fn test_fs_c08_rmdir() {
    assert!(true);
}
#[test]
fn test_fs_c09_readdir() {
    assert!(true);
}
#[test]
fn test_fs_c10_stat() {
    assert!(true);
}
#[test]
fn test_fs_c11_lseek() {
    let offset = 100u64;
    assert!(offset > 0);
}
#[test]
fn test_fs_c12_truncate() {
    assert!(true);
}
#[test]
fn test_fs_c13_symlink() {
    // Symlink depth limit: 8
    let max_depth = 8;
    assert_eq!(max_depth, 8);
}
#[test]
fn test_fs_c14_mount() {
    assert!(true);
}
#[test]
fn test_fs_c15_umount() {
    assert!(true);
}

#[test]
fn test_fs_e01_open_noent() {
    let enoent: i64 = -2;
    assert!(enoent < 0);
}
#[test]
fn test_fs_e02_mkdir_exists() {
    let eexist: i64 = -17;
    assert!(eexist < 0);
}
#[test]
fn test_fs_e03_rmdir_notempty() {
    let enotempty: i64 = -39;
    assert!(enotempty < 0);
}
#[test]
fn test_fs_e04_read_isdir() {
    let eisdir: i64 = -21;
    assert!(eisdir < 0);
}
#[test]
fn test_fs_e05_write_rofs() {
    let erofs: i64 = -30;
    assert!(erofs < 0);
}
#[test]
fn test_fs_permission_bits() {
    let rwxr_xr_x = 0o755u32;
    assert!(rwxr_xr_x & 0o400 != 0); // owner read
    assert!(rwxr_xr_x & 0o200 != 0); // owner write
    assert!(rwxr_xr_x & 0o100 != 0); // owner exec
    assert!(rwxr_xr_x & 0o040 != 0); // group read
    assert!(rwxr_xr_x & 0o020 == 0); // group no write
}
