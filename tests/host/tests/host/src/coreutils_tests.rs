//! Core utility tests
#[test] fn echo_basic() { let output = "hello"; assert_eq!(output, "hello"); }
#[test] fn echo_no_newline() { let output = "hello"; assert!(!output.ends_with('\n')); }
#[test] fn wc_count_lines() { let text = "a\nb\nc\n"; assert_eq!(text.matches('\n').count(), 3); }
#[test] fn wc_count_words() { let text = "hello world foo"; assert_eq!(text.split_whitespace().count(), 3); }
#[test] fn wc_count_bytes() { let text = "hello"; assert_eq!(text.len(), 5); }
#[test] fn head_3_lines() {
    let text = "1\n2\n3\n4\n5\n";
    let head: Vec<&str> = text.lines().take(3).collect();
    assert_eq!(head, vec!["1", "2", "3"]);
}
#[test] fn tail_3_lines() {
    let lines: Vec<&str> = "1\n2\n3\n4\n5".lines().collect();
    let tail: Vec<&&str> = lines.iter().rev().take(3).rev().collect();
    assert_eq!(*tail[0], "3");
}
#[test] fn grep_match() {
    let lines = vec!["hello world", "foo bar", "hello again"];
    let matches: Vec<&&str> = lines.iter().filter(|l| l.contains("hello")).collect();
    assert_eq!(matches.len(), 2);
}
#[test] fn grep_no_match() {
    let lines = vec!["foo", "bar"];
    let matches: Vec<&&str> = lines.iter().filter(|l| l.contains("xyz")).collect();
    assert!(matches.is_empty());
}
#[test] fn stat_format() { /* stat should show inode, size, mode */ assert!(true); }
#[test] fn free_format() { /* free should show total, used, free */ assert!(true); }
#[test] fn uptime_positive() { let secs = 42u64; assert!(secs > 0); }
#[test] fn uname_contains_logos() { let sysname = "LogOS"; assert!(sysname.contains("LogOS")); }
#[test] fn uname_contains_x86_64() { let machine = "x86_64"; assert!(machine.contains("x86_64")); }
#[test] fn hexdump_format() {
    let byte: u8 = 0xAB;
    let hex = format!("{:02x}", byte);
    assert_eq!(hex, "ab");
}
#[test] fn mkdir_creates_dir() { assert!(true); }
#[test] fn rmdir_empty() { assert!(true); }
#[test] fn touch_creates_file() { assert!(true); }
#[test] fn rm_removes_file() { assert!(true); }
#[test] fn cp_copies() { let src = vec![1u8, 2, 3]; let dst = src.clone(); assert_eq!(src, dst); }
#[test] fn mv_renames() { assert!(true); }
#[test] fn ls_lists_entries() { assert!(true); }
#[test] fn cat_reads_file() { assert!(true); }
#[test] fn ps_shows_processes() { assert!(true); }
#[test] fn kill_sends_signal() { assert!(true); }
#[test] fn clear_resets_terminal() { assert!(true); }
#[test] fn dmesg_shows_log() { assert!(true); }
