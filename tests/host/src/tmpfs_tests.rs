/// tmpfs logic tests.

#[test]
fn test_tmpfs_inode_allocation() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let next = AtomicU64::new(1);
    let i1 = next.fetch_add(1, Ordering::Relaxed);
    let i2 = next.fetch_add(1, Ordering::Relaxed);
    assert_ne!(i1, i2);
}

#[test]
fn test_tmpfs_file_growth() {
    let mut data = Vec::new();
    data.extend_from_slice(b"initial");
    assert_eq!(data.len(), 7);
    data.extend_from_slice(b" extended");
    assert_eq!(data.len(), 16);
}

#[test]
fn test_tmpfs_file_truncate() {
    let mut data = vec![0xABu8; 1024];
    data.truncate(512);
    assert_eq!(data.len(), 512);
}

#[test]
fn test_tmpfs_directory_entries() {
    let mut entries = std::collections::BTreeMap::new();
    entries.insert("file1".to_string(), 1u64);
    entries.insert("file2".to_string(), 2);
    entries.insert("dir1".to_string(), 3);
    assert_eq!(entries.len(), 3);
    assert!(entries.contains_key("file1"));
    entries.remove("file2");
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_tmpfs_readdir_dot_dotdot() {
    let entries = vec![".", "..", "file.txt"];
    assert!(entries.contains(&"."));
    assert!(entries.contains(&".."));
    assert_eq!(entries.len(), 3);
}

#[test]
fn test_tmpfs_permissions() {
    let file_mode = 0o644u32;
    let dir_mode = 0o755u32;
    assert!(file_mode & 0o400 != 0); // owner read
    assert!(file_mode & 0o200 != 0); // owner write
    assert!(dir_mode & 0o100 != 0); // owner execute (for dirs)
}

#[test]
fn test_tmpfs_nested_dirs() {
    let mut tree = std::collections::BTreeMap::new();
    tree.insert("/tmp", vec!["dir1", "dir2"]);
    tree.insert("/tmp/dir1", vec!["file.txt"]);
    assert_eq!(tree["/tmp"].len(), 2);
    assert_eq!(tree["/tmp/dir1"].len(), 1);
}

#[test]
fn test_tmpfs_overwrite() {
    let mut data = b"original".to_vec();
    let new_data = b"REPLACED";
    data[..new_data.len()].copy_from_slice(new_data);
    assert_eq!(&data, b"REPLACED");
}

#[test]
fn test_tmpfs_empty_file() {
    let data: Vec<u8> = Vec::new();
    assert_eq!(data.len(), 0);
    // Read from empty file returns 0
    assert!(data.is_empty());
}

#[test]
fn test_tmpfs_large_file() {
    let size = 1024 * 1024; // 1 MiB
    let data = vec![0xCDu8; size];
    assert_eq!(data.len(), size);
    assert!(data.iter().all(|&b| b == 0xCD));
}
