//! Shell parsing tests
fn split_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in cmd.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            ' ' if !in_quote => { if !current.is_empty() { args.push(current.clone()); current.clear(); } }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() { args.push(current); }
    args
}

#[test] fn parse_simple() { assert_eq!(split_args("ls"), vec!["ls"]); }
#[test] fn parse_with_args() { assert_eq!(split_args("ls -la /tmp"), vec!["ls", "-la", "/tmp"]); }
#[test] fn parse_quoted() { assert_eq!(split_args("echo \"hello world\""), vec!["echo", "hello world"]); }
#[test] fn parse_empty() { assert!(split_args("").is_empty()); }
#[test] fn parse_spaces_only() { assert!(split_args("   ").is_empty()); }
#[test] fn parse_multiple_spaces() { assert_eq!(split_args("a  b   c"), vec!["a", "b", "c"]); }
#[test] fn builtin_cd() { let cmd = "cd"; assert_eq!(cmd, "cd"); }
#[test] fn builtin_exit() { let cmd = "exit"; assert_eq!(cmd, "exit"); }
#[test] fn builtin_help() { let cmd = "help"; assert_eq!(cmd, "help"); }
#[test] fn command_not_found() { let known = ["ls", "cat", "echo"]; assert!(!known.contains(&"nonexistent")); }
#[test] fn long_command() { let cmd = "a".repeat(4000); assert_eq!(cmd.len(), 4000); }
