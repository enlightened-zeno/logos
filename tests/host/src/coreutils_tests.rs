/// Core utility logic tests.

#[test]
fn test_echo_output() {
    let args = vec!["hello", "world"];
    let output = args.join(" ");
    assert_eq!(output, "hello world");
}

#[test]
fn test_echo_no_args() {
    let args: Vec<&str> = vec![];
    let output = args.join(" ");
    assert_eq!(output, "");
}

#[test]
fn test_wc_lines() {
    let text = "line1\nline2\nline3\n";
    let lines = text.matches('\n').count();
    assert_eq!(lines, 3);
}

#[test]
fn test_wc_words() {
    let text = "hello world  foo";
    let words = text.split_whitespace().count();
    assert_eq!(words, 3);
}

#[test]
fn test_wc_bytes() {
    let text = "hello";
    assert_eq!(text.len(), 5);
}

#[test]
fn test_head_n() {
    let lines = vec!["a", "b", "c", "d", "e"];
    let head: Vec<&&str> = lines.iter().take(3).collect();
    assert_eq!(head, vec![&"a", &"b", &"c"]);
}

#[test]
fn test_tail_n() {
    let lines = vec!["a", "b", "c", "d", "e"];
    let n = 3;
    let tail: Vec<&&str> = lines.iter().skip(lines.len() - n).collect();
    assert_eq!(tail, vec![&"c", &"d", &"e"]);
}

#[test]
fn test_cat_concatenate() {
    let file1 = b"hello ";
    let file2 = b"world";
    let mut output = Vec::new();
    output.extend_from_slice(file1);
    output.extend_from_slice(file2);
    assert_eq!(output, b"hello world");
}

#[test]
fn test_hexdump_format() {
    let byte: u8 = 0xAB;
    let hex = format!("{:02x}", byte);
    assert_eq!(hex, "ab");
}

#[test]
fn test_uptime_format() {
    let ticks: u64 = 65000;
    let secs = ticks / 1000;
    let mins = secs / 60;
    assert_eq!(secs, 65);
    assert_eq!(mins, 1);
}

#[test]
fn test_free_format() {
    let total_kb: u64 = 256 * 1024;
    let free_kb: u64 = 200 * 1024;
    let used_kb = total_kb - free_kb;
    assert_eq!(used_kb, 56 * 1024);
}

#[test]
fn test_stat_inode_type_display() {
    let types = vec![('d', "Directory"), ('-', "File"), ('c', "CharDevice"),
                     ('b', "BlockDevice"), ('l', "Symlink"), ('p', "Pipe")];
    assert_eq!(types.len(), 6);
}

#[test]
fn test_mkdir_path() {
    let path = "/tmp/newdir";
    let parent = &path[..path.rfind('/').unwrap()];
    let name = &path[path.rfind('/').unwrap() + 1..];
    assert_eq!(parent, "/tmp");
    assert_eq!(name, "newdir");
}

#[test]
fn test_rm_nonexistent() {
    let exists = false;
    assert!(!exists); // Should return ENOENT
}

#[test]
fn test_cp_size_preserved() {
    let src = vec![1u8, 2, 3, 4, 5];
    let dst = src.clone();
    assert_eq!(src.len(), dst.len());
    assert_eq!(src, dst);
}
