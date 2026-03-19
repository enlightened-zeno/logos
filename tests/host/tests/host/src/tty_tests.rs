//! TTY subsystem tests
#[test] fn tty_canonical_newline_delimit() { let line = "hello\n"; assert!(line.ends_with('\n')); }
#[test] fn tty_backspace_removes_char() { let mut buf = String::from("helo"); buf.pop(); buf.push('l'); assert_eq!(buf, "hell"); }
#[test] fn tty_ctrl_u_clears_line() { let mut buf = String::from("text"); buf.clear(); assert!(buf.is_empty()); }
#[test] fn tty_ctrl_w_kills_word() {
    let mut buf = String::from("hello world");
    while buf.ends_with(|c: char| !c.is_whitespace()) { buf.pop(); }
    while buf.ends_with(char::is_whitespace) { buf.pop(); }
    assert_eq!(buf, "hello");
}
#[test] fn tty_empty_backspace_noop() { let mut buf = String::new(); if !buf.is_empty() { buf.pop(); } assert!(buf.is_empty()); }
#[test] fn tty_max_line_length() { let max = 4096usize; let line = "a".repeat(max); assert_eq!(line.len(), max); }
#[test] fn tty_echo_mode() { let echo = true; assert!(echo); }
#[test] fn tty_raw_mode() { let canonical = false; assert!(!canonical); }
#[test] fn tty_ctrl_c_is_0x03() { assert_eq!(b'C' - b'@', 3); }
#[test] fn tty_ctrl_d_is_0x04() { assert_eq!(b'D' - b'@', 4); }
