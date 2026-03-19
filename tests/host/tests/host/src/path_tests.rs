//! Path normalization and manipulation tests

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => { parts.pop(); }
            p => parts.push(p),
        }
    }
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(i) => &path[..i],
        None => ".",
    }
}

#[test]
fn normalize_root() { assert_eq!(normalize("/"), "/"); }
#[test]
fn normalize_simple() { assert_eq!(normalize("/a/b/c"), "/a/b/c"); }
#[test]
fn normalize_trailing_slash() { assert_eq!(normalize("/a/b/"), "/a/b"); }
#[test]
fn normalize_double_slash() { assert_eq!(normalize("/a//b"), "/a/b"); }
#[test]
fn normalize_dot() { assert_eq!(normalize("/a/./b"), "/a/b"); }
#[test]
fn normalize_dotdot() { assert_eq!(normalize("/a/b/../c"), "/a/c"); }
#[test]
fn normalize_dotdot_root() { assert_eq!(normalize("/.."), "/"); }
#[test]
fn normalize_complex() { assert_eq!(normalize("/a/b/c/../../d"), "/a/d"); }
#[test]
fn basename_file() { assert_eq!(basename("/a/b/c.txt"), "c.txt"); }
#[test]
fn basename_dir() { assert_eq!(basename("/a/b/c"), "c"); }
#[test]
fn basename_root() { assert_eq!(basename("/"), ""); }
#[test]
fn parent_file() { assert_eq!(parent("/a/b/c"), "/a/b"); }
#[test]
fn parent_root() { assert_eq!(parent("/a"), "/"); }
#[test]
fn parent_no_slash() { assert_eq!(parent("file"), "."); }
#[test]
fn normalize_many_dots() { assert_eq!(normalize("/a/b/c/./././d"), "/a/b/c/d"); }
#[test]
fn normalize_all_dotdot() { assert_eq!(normalize("/a/b/c/../../../"), "/"); }
#[test]
fn path_split_components() {
    let path = "/usr/bin/ls";
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    assert_eq!(parts, vec!["usr", "bin", "ls"]);
}
#[test]
fn empty_path_normalize() { assert_eq!(normalize(""), "/"); }
#[test]
fn just_slash() { assert_eq!(normalize("////"), "/"); }
