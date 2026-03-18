use crate::arch::x86_64::io;
use core::sync::atomic::{AtomicBool, Ordering};

const KBD_DATA_PORT: u16 = 0x60;

/// Modifier key state.
static SHIFT_HELD: AtomicBool = AtomicBool::new(false);
static CTRL_HELD: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);

/// Keyboard input ring buffer.
const BUF_SIZE: usize = 256;
static mut KEY_BUF: [u8; BUF_SIZE] = [0; BUF_SIZE];
static mut BUF_HEAD: usize = 0;
static mut BUF_TAIL: usize = 0;

/// Push a character into the keyboard buffer.
fn buf_push(ch: u8) {
    // SAFETY: Only called from the keyboard ISR (single context).
    unsafe {
        let next = (BUF_HEAD + 1) % BUF_SIZE;
        if next != BUF_TAIL {
            KEY_BUF[BUF_HEAD] = ch;
            BUF_HEAD = next;
        }
    }
}

/// Pop a character from the keyboard buffer. Returns None if empty.
#[allow(dead_code)]
pub fn buf_pop() -> Option<u8> {
    // SAFETY: Tail is only advanced here. Head is only advanced in ISR.
    // A missed update is benign (we just don't see the char yet).
    unsafe {
        if BUF_TAIL == BUF_HEAD {
            return None;
        }
        let ch = KEY_BUF[BUF_TAIL];
        BUF_TAIL = (BUF_TAIL + 1) % BUF_SIZE;
        Some(ch)
    }
}

/// Called from the keyboard interrupt handler.
pub fn handle_scancode() {
    let scancode = io::inb(KBD_DATA_PORT);

    // Scan Code Set 1 translation
    let released = scancode & 0x80 != 0;
    let code = scancode & 0x7F;

    match code {
        0x2A | 0x36 => {
            // Left/Right Shift
            SHIFT_HELD.store(!released, Ordering::Relaxed);
            return;
        }
        0x1D => {
            // Left Ctrl
            CTRL_HELD.store(!released, Ordering::Relaxed);
            return;
        }
        0x3A if !released => {
            // Caps Lock toggle
            let current = CAPS_LOCK.load(Ordering::Relaxed);
            CAPS_LOCK.store(!current, Ordering::Relaxed);
            return;
        }
        _ => {}
    }

    if released {
        return;
    }

    let shift = SHIFT_HELD.load(Ordering::Relaxed);
    let ctrl = CTRL_HELD.load(Ordering::Relaxed);
    let caps = CAPS_LOCK.load(Ordering::Relaxed);

    let ch = scancode_to_ascii(code, shift, caps);

    if let Some(mut ch) = ch {
        if ctrl {
            // Ctrl+A..Z → 1..26
            if ch.is_ascii_lowercase() {
                ch = (ch as u8 - b'a' + 1) as char;
            } else if ch.is_ascii_uppercase() {
                ch = (ch as u8 - b'A' + 1) as char;
            }
        }
        buf_push(ch as u8);
    }
}

/// Translate Scan Code Set 1 to ASCII.
fn scancode_to_ascii(code: u8, shift: bool, caps: bool) -> Option<char> {
    let base = match code {
        0x02 => Some(('1', '!')),
        0x03 => Some(('2', '@')),
        0x04 => Some(('3', '#')),
        0x05 => Some(('4', '$')),
        0x06 => Some(('5', '%')),
        0x07 => Some(('6', '^')),
        0x08 => Some(('7', '&')),
        0x09 => Some(('8', '*')),
        0x0A => Some(('9', '(')),
        0x0B => Some(('0', ')')),
        0x0C => Some(('-', '_')),
        0x0D => Some(('=', '+')),
        0x0E => Some(('\x08', '\x08')), // Backspace
        0x0F => Some(('\t', '\t')),     // Tab
        0x10 => Some(('q', 'Q')),
        0x11 => Some(('w', 'W')),
        0x12 => Some(('e', 'E')),
        0x13 => Some(('r', 'R')),
        0x14 => Some(('t', 'T')),
        0x15 => Some(('y', 'Y')),
        0x16 => Some(('u', 'U')),
        0x17 => Some(('i', 'I')),
        0x18 => Some(('o', 'O')),
        0x19 => Some(('p', 'P')),
        0x1A => Some(('[', '{')),
        0x1B => Some((']', '}')),
        0x1C => Some(('\n', '\n')), // Enter
        0x1E => Some(('a', 'A')),
        0x1F => Some(('s', 'S')),
        0x20 => Some(('d', 'D')),
        0x21 => Some(('f', 'F')),
        0x22 => Some(('g', 'G')),
        0x23 => Some(('h', 'H')),
        0x24 => Some(('j', 'J')),
        0x25 => Some(('k', 'K')),
        0x26 => Some(('l', 'L')),
        0x27 => Some((';', ':')),
        0x28 => Some(('\'', '"')),
        0x29 => Some(('`', '~')),
        0x2B => Some(('\\', '|')),
        0x2C => Some(('z', 'Z')),
        0x2D => Some(('x', 'X')),
        0x2E => Some(('c', 'C')),
        0x2F => Some(('v', 'V')),
        0x30 => Some(('b', 'B')),
        0x31 => Some(('n', 'N')),
        0x32 => Some(('m', 'M')),
        0x33 => Some((',', '<')),
        0x34 => Some(('.', '>')),
        0x35 => Some(('/', '?')),
        0x39 => Some((' ', ' ')), // Space
        _ => None,
    };

    base.map(|(lower, upper)| {
        let use_upper = if lower.is_ascii_alphabetic() {
            shift ^ caps
        } else {
            shift
        };
        if use_upper {
            upper
        } else {
            lower
        }
    })
}
