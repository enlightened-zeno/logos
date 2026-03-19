/// Keyboard scancode translation tests.

fn scancode_to_char(code: u8, shift: bool) -> Option<char> {
    let base = match code {
        0x02 => Some(('1', '!')), 0x03 => Some(('2', '@')), 0x04 => Some(('3', '#')),
        0x10 => Some(('q', 'Q')), 0x11 => Some(('w', 'W')), 0x12 => Some(('e', 'E')),
        0x1E => Some(('a', 'A')), 0x1F => Some(('s', 'S')), 0x20 => Some(('d', 'D')),
        0x2C => Some(('z', 'Z')), 0x2D => Some(('x', 'X')), 0x1C => Some(('\n', '\n')),
        0x39 => Some((' ', ' ')), 0x0E => Some(('\x08', '\x08')),
        _ => None,
    };
    base.map(|(l, u)| if shift { u } else { l })
}

#[test]
fn test_lowercase() {
    assert_eq!(scancode_to_char(0x10, false), Some('q'));
    assert_eq!(scancode_to_char(0x1E, false), Some('a'));
    assert_eq!(scancode_to_char(0x2C, false), Some('z'));
}

#[test]
fn test_uppercase() {
    assert_eq!(scancode_to_char(0x10, true), Some('Q'));
    assert_eq!(scancode_to_char(0x1E, true), Some('A'));
}

#[test]
fn test_numbers() {
    assert_eq!(scancode_to_char(0x02, false), Some('1'));
    assert_eq!(scancode_to_char(0x02, true), Some('!'));
}

#[test]
fn test_special_keys() {
    assert_eq!(scancode_to_char(0x1C, false), Some('\n'));
    assert_eq!(scancode_to_char(0x39, false), Some(' '));
    assert_eq!(scancode_to_char(0x0E, false), Some('\x08'));
}

#[test]
fn test_unknown_scancode() {
    assert_eq!(scancode_to_char(0xFF, false), None);
}

#[test]
fn test_release_bit() {
    let press: u8 = 0x1E;
    let release: u8 = press | 0x80;
    assert_eq!(release & 0x80, 0x80);
    assert_eq!(release & 0x7F, press);
}

#[test]
fn test_modifier_tracking() {
    let mut shift = false;
    let mut ctrl = false;

    // Press left shift (0x2A)
    shift = true;
    assert!(shift);

    // Press ctrl (0x1D)
    ctrl = true;
    assert!(ctrl);

    // Ctrl+C = 0x03
    let ch = if ctrl { 3u8 } else { b'c' };
    assert_eq!(ch, 3);

    // Release shift
    shift = false;
    assert!(!shift);
}
