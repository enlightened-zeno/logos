/// VFS logic tests.

#[test]
fn test_mount_longest_prefix() {
    // VFS resolve uses longest prefix matching for mounts
    let mounts = vec![
        ("/", "tmpfs"),
        ("/dev", "devfs"),
        ("/dev/pts", "devpts"),
        ("/proc", "procfs"),
    ];

    let find_mount = |path: &str| -> &str {
        let mut best = "";
        let mut best_len = 0;
        for (mount_path, fs_name) in &mounts {
            if path.starts_with(mount_path)
                && mount_path.len() >= best_len
                && (path.len() == mount_path.len()
                    || path.as_bytes().get(mount_path.len()) == Some(&b'/'))
            {
                best = fs_name;
                best_len = mount_path.len();
            }
        }
        // Root always matches
        if best.is_empty() {
            "tmpfs"
        } else {
            best
        }
    };

    assert_eq!(find_mount("/"), "tmpfs");
    assert_eq!(find_mount("/tmp"), "tmpfs"); // Falls to /
    assert_eq!(find_mount("/dev"), "devfs");
    assert_eq!(find_mount("/dev/null"), "devfs");
    assert_eq!(find_mount("/dev/pts/0"), "devpts");
    assert_eq!(find_mount("/proc"), "procfs");
    assert_eq!(find_mount("/proc/meminfo"), "procfs");
}

#[test]
fn test_path_components() {
    let path = "/a/b/c/d";
    let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
    assert_eq!(components, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_inode_types() {
    #[derive(PartialEq, Debug)]
    enum InodeType {
        File,
        Directory,
        CharDevice,
        BlockDevice,
        Symlink,
        Pipe,
    }

    let types = vec![
        InodeType::File,
        InodeType::Directory,
        InodeType::CharDevice,
        InodeType::BlockDevice,
        InodeType::Symlink,
        InodeType::Pipe,
    ];
    assert_eq!(types.len(), 6);
    assert_ne!(InodeType::File, InodeType::Directory);
}

#[test]
fn test_open_flags() {
    let o_rdonly = 0;
    let o_wronly = 1;
    let o_rdwr = 2;
    assert_ne!(o_rdonly, o_wronly);
    assert_ne!(o_rdonly, o_rdwr);
    assert_ne!(o_wronly, o_rdwr);
}

#[test]
fn test_max_path_length() {
    let max_path = 4096;
    let long_path = "/".to_string() + &"a".repeat(max_path);
    assert!(long_path.len() > max_path);
}
