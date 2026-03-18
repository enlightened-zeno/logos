extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub mod builtins;

const PROMPT: &str = "logos# ";
const MAX_LINE: usize = 1024;

/// Run the interactive kernel shell. This function never returns.
pub fn run() -> ! {
    crate::serial_print!("\nLogoOS v0.1.0 — kernel shell\n");
    crate::serial_print!("Type 'help' for available commands.\n\n");

    loop {
        crate::serial_print!("{}", PROMPT);

        let line = read_line();
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let (cmd, args) = parse_line(line);
        execute(&cmd, &args);
    }
}

/// Read a line from the TTY, blocking until a newline is received.
fn read_line() -> String {
    let mut buf = [0u8; MAX_LINE];
    let mut pos = 0;

    loop {
        if !crate::tty::has_input() {
            crate::arch::x86_64::cpu::hlt();
            continue;
        }

        let n = crate::tty::read(&mut buf[pos..]);
        if n == 0 {
            continue;
        }

        pos += n;

        // Check if we got a newline
        if pos > 0 && buf[pos - 1] == b'\n' {
            return String::from(core::str::from_utf8(&buf[..pos - 1]).unwrap_or(""));
        }

        if pos >= MAX_LINE {
            return String::from(core::str::from_utf8(&buf[..pos]).unwrap_or(""));
        }
    }
}

/// Parse a command line into command name and arguments.
fn parse_line(line: &str) -> (String, Vec<String>) {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in line.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    parts.push(core::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        return (String::new(), Vec::new());
    }

    let cmd = parts.remove(0);
    (cmd, parts)
}

/// Execute a command.
fn execute(cmd: &str, args: &[String]) {
    match cmd {
        "help" => builtins::help(),
        "echo" => builtins::echo(args),
        "ls" => builtins::ls(args),
        "cat" => builtins::cat(args),
        "mkdir" => builtins::mkdir(args),
        "touch" => builtins::touch(args),
        "rm" => builtins::rm(args),
        "write" => builtins::write_file(args),
        "pwd" => builtins::pwd(),
        "cd" => builtins::cd(args),
        "stat" => builtins::stat(args),
        "free" => builtins::free(),
        "uptime" => builtins::uptime(),
        "uname" => builtins::uname(),
        "ps" => builtins::ps(),
        "clear" => builtins::clear(),
        "hexdump" => builtins::hexdump(args),
        "dmesg" => builtins::dmesg(),
        "shutdown" | "halt" => builtins::shutdown(),
        "reboot" => builtins::reboot(),
        "pipe-test" => builtins::pipe_test(),
        "stress" => builtins::stress_test(args),
        "bench" => builtins::bench(args),
        "leakcheck" => builtins::leak_check(),
        _ => {
            crate::serial_println!("lsh: {}: command not found", cmd);
        }
    }
}
