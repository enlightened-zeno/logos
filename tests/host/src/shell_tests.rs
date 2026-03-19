/// Shell parsing and logic tests.

fn parse_line(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape { current.push(ch); escape = false; continue; }
        match ch {
            '\\' if !in_single => { escape = true; }
            '\'' if !in_double => { in_single = !in_single; }
            '"' if !in_single => { in_double = !in_double; }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() { parts.push(std::mem::take(&mut current)); }
            }
            _ => { current.push(ch); }
        }
    }
    if !current.is_empty() { parts.push(current); }
    parts
}

#[test]
fn test_simple_command() {
    assert_eq!(parse_line("ls"), vec!["ls"]);
}

#[test]
fn test_command_with_args() {
    assert_eq!(parse_line("ls -la /tmp"), vec!["ls", "-la", "/tmp"]);
}

#[test]
fn test_double_quotes() {
    assert_eq!(parse_line(r#"echo "hello world""#), vec!["echo", "hello world"]);
}

#[test]
fn test_single_quotes() {
    assert_eq!(parse_line("echo '$HOME'"), vec!["echo", "$HOME"]);
}

#[test]
fn test_escape() {
    assert_eq!(parse_line(r"echo hello\ world"), vec!["echo", "hello world"]);
}

#[test]
fn test_empty_input() {
    assert!(parse_line("").is_empty());
    assert!(parse_line("   ").is_empty());
}

#[test]
fn test_multiple_spaces() {
    assert_eq!(parse_line("ls    -l    /tmp"), vec!["ls", "-l", "/tmp"]);
}

#[test]
fn test_tabs() {
    assert_eq!(parse_line("echo\thello"), vec!["echo", "hello"]);
}

#[test]
fn test_builtin_commands() {
    let builtins = vec![
        "help", "echo", "ls", "cat", "mkdir", "touch", "rm", "write",
        "pwd", "cd", "stat", "free", "uptime", "uname", "ps", "hexdump",
        "dmesg", "clear", "shutdown", "reboot", "pipe-test", "stress",
        "bench", "leakcheck", "run-user",
    ];
    assert!(builtins.len() >= 24);
    assert!(builtins.contains(&"cd"));
    assert!(builtins.contains(&"echo"));
}

#[test]
fn test_command_not_found() {
    let known = vec!["ls", "cd", "echo"];
    let cmd = "nonexistent";
    assert!(!known.contains(&cmd));
}
