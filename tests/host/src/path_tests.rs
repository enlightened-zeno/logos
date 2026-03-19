/// Path normalization tests (mirrors kernel/src/fs/path.rs logic).
/// Since we can't import the kernel crate on host, we duplicate the logic.

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let absolute = path.starts_with('/');
    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                if !parts.is_empty() && *parts.last().unwrap() != ".." {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let mut result = if absolute {
        String::from("/")
    } else {
        String::new()
    };
    for (i, part) in parts.iter().enumerate() {
        if (i > 0 || absolute) && !(absolute && i == 0) {
            result.push('/');
        }
        result.push_str(part);
    }
    if result.is_empty() {
        return if absolute {
            String::from("/")
        } else {
            String::from(".")
        };
    }
    result
}

fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(pos) => &path[pos + 1..],
        None => path,
    }
}

fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(pos) => &path[..pos],
        None => ".",
    }
}

#[test]
fn test_normalize_absolute() {
    assert_eq!(normalize("/a/b/c"), "/a/b/c");
    assert_eq!(normalize("/a//b"), "/a/b");
    assert_eq!(normalize("/a/./b"), "/a/b");
    assert_eq!(normalize("/"), "/");
    assert_eq!(normalize("/."), "/");
    assert_eq!(normalize("/.."), "/");
}

#[test]
fn test_normalize_dotdot() {
    assert_eq!(normalize("/a/b/../c"), "/a/c");
    assert_eq!(normalize("/a/b/../../c"), "/c");
    assert_eq!(normalize("/../../../tmp"), "/tmp");
    assert_eq!(normalize("/a/../b/../c"), "/c");
}

#[test]
fn test_normalize_trailing_slash() {
    assert_eq!(normalize("/a/b/"), "/a/b");
    assert_eq!(normalize("/a/b/c/"), "/a/b/c");
}

#[test]
fn test_normalize_multiple_slashes() {
    assert_eq!(normalize("///a///b///c"), "/a/b/c");
    assert_eq!(normalize("//"), "/");
}

#[test]
fn test_basename() {
    assert_eq!(basename("/a/b/c.txt"), "c.txt");
    assert_eq!(basename("file.rs"), "file.rs");
    assert_eq!(basename("/a/b/"), "");
    assert_eq!(basename("/"), "");
}

#[test]
fn test_parent() {
    assert_eq!(parent("/a/b/c"), "/a/b");
    assert_eq!(parent("/a"), "/");
    assert_eq!(parent("file.rs"), ".");
}
