extern crate alloc;

use crate::sync::SpinLock;

/// TTY line buffer size.
const LINE_BUF_SIZE: usize = 4096;

/// TTY modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtyMode {
    /// Canonical (cooked) mode: line editing, echo, signal chars.
    Canonical,
    /// Raw mode: characters delivered immediately, no processing.
    Raw,
}

/// Input ring buffer for cooked characters ready for reading.
const INPUT_BUF_SIZE: usize = 4096;

struct TtyInner {
    mode: TtyMode,
    echo: bool,
    /// Line editing buffer (canonical mode).
    line_buf: [u8; LINE_BUF_SIZE],
    line_len: usize,
    /// Cooked input buffer (ready for read()).
    input_buf: [u8; INPUT_BUF_SIZE],
    input_head: usize,
    input_tail: usize,
    input_count: usize,
}

static TTY: SpinLock<TtyInner> = SpinLock::new(TtyInner {
    mode: TtyMode::Canonical,
    echo: true,
    line_buf: [0; LINE_BUF_SIZE],
    line_len: 0,
    input_buf: [0; INPUT_BUF_SIZE],
    input_head: 0,
    input_tail: 0,
    input_count: 0,
});

/// Initialize the TTY subsystem.
pub fn init() {
    crate::serial_println!("TTY: initialized (canonical mode)");
}

/// Process a character from the keyboard.
///
/// In canonical mode: line editing with echo.
/// In raw mode: character delivered directly to input buffer.
pub fn input_char(ch: u8) {
    let mut tty = TTY.lock();

    match tty.mode {
        TtyMode::Raw => {
            push_input(&mut tty, ch);
        }
        TtyMode::Canonical => {
            match ch {
                // Ctrl+C → SIGINT (for now, just print ^C)
                3 => {
                    if tty.echo {
                        write_output(b"^C\n");
                    }
                    tty.line_len = 0;
                }
                // Ctrl+D → EOF (submit empty line when buffer is empty)
                4 if tty.line_len == 0 => {
                    push_input(&mut tty, 0);
                }
                // Ctrl+U → kill line
                21 => {
                    if tty.echo {
                        // Erase the visible line
                        for _ in 0..tty.line_len {
                            write_output(b"\x08 \x08");
                        }
                    }
                    tty.line_len = 0;
                }
                // Ctrl+W → delete word
                23 => {
                    // Delete trailing spaces, then non-spaces
                    while tty.line_len > 0 && tty.line_buf[tty.line_len - 1] == b' ' {
                        tty.line_len -= 1;
                        if tty.echo {
                            write_output(b"\x08 \x08");
                        }
                    }
                    while tty.line_len > 0 && tty.line_buf[tty.line_len - 1] != b' ' {
                        tty.line_len -= 1;
                        if tty.echo {
                            write_output(b"\x08 \x08");
                        }
                    }
                }
                // Backspace
                0x08 | 0x7F if tty.line_len > 0 => {
                    tty.line_len -= 1;
                    if tty.echo {
                        write_output(b"\x08 \x08");
                    }
                }
                // Enter → submit line
                b'\n' | b'\r' => {
                    if tty.echo {
                        write_output(b"\n");
                    }
                    // Copy line buffer to input buffer
                    let len = tty.line_len;
                    for i in 0..len {
                        let byte = tty.line_buf[i];
                        push_input(&mut tty, byte);
                    }
                    push_input(&mut tty, b'\n');
                    tty.line_len = 0;
                }
                // Regular character
                ch if (0x20..0x7F).contains(&ch) => {
                    let pos = tty.line_len;
                    if pos < LINE_BUF_SIZE {
                        tty.line_buf[pos] = ch;
                        tty.line_len = pos + 1;
                        if tty.echo {
                            write_output(&[ch]);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Read from the TTY input buffer. Returns bytes read.
pub fn read(buf: &mut [u8]) -> usize {
    let mut tty = TTY.lock();
    let mut count = 0;

    while count < buf.len() && tty.input_count > 0 {
        let ch = tty.input_buf[tty.input_tail];
        tty.input_tail = (tty.input_tail + 1) % INPUT_BUF_SIZE;
        tty.input_count -= 1;

        if ch == 0 {
            // EOF sentinel
            break;
        }

        buf[count] = ch;
        count += 1;

        // In canonical mode, stop at newline
        if tty.mode == TtyMode::Canonical && ch == b'\n' {
            break;
        }
    }

    count
}

/// Check if there is input available.
pub fn has_input() -> bool {
    TTY.lock().input_count > 0
}

/// Write output to the console (serial port for now).
fn write_output(data: &[u8]) {
    for &byte in data {
        crate::drivers::serial::write_byte(byte);
    }
}

/// Push a byte into the cooked input buffer.
fn push_input(tty: &mut TtyInner, ch: u8) {
    if tty.input_count < INPUT_BUF_SIZE {
        tty.input_buf[tty.input_head] = ch;
        tty.input_head = (tty.input_head + 1) % INPUT_BUF_SIZE;
        tty.input_count += 1;
    }
}
