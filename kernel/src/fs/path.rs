extern crate alloc;

use alloc::string::String;

/// Normalize a path: resolve "." and "..", collapse multiple slashes.
pub fn normalize(path: &str) -> String {
    let mut parts: alloc::vec::Vec<&str> = alloc::vec::Vec::new();
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
        if absolute {
            return String::from("/");
        }
        return String::from(".");
    }

    result
}

/// Get the parent directory of a path.
pub fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(pos) => &path[..pos],
        None => ".",
    }
}

/// Get the file name component of a path.
pub fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(pos) => &path[pos + 1..],
        None => path,
    }
}
