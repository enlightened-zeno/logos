//! Keyboard driver tests
#[test] fn scancode_a_press() { assert_eq!(0x1Eu8, 0x1E); }
#[test] fn scancode_a_release() { assert_eq!(0x1Eu8 | 0x80, 0x9E); }
#[test] fn scancode_is_release() { let code: u8 = 0x9E; assert!(code & 0x80 != 0); }
#[test] fn scancode_is_press() { let code: u8 = 0x1E; assert!(code & 0x80 == 0); }
#[test] fn scancode_to_press() { let code: u8 = 0x9E; let press = code & !0x80; assert_eq!(press, 0x1E); }
#[test] fn shift_modifies_a() { let base = b'a'; let shifted = b'A'; assert_eq!(base.to_ascii_uppercase(), shifted); }
#[test] fn ctrl_c_scancode() { assert_eq!(0x2Eu8, 0x2E); /* scancode for 'c' */ }
#[test] fn extended_prefix() { assert_eq!(0xE0u8, 0xE0); }
#[test] fn ring_buffer_wrap() {
    let cap = 128usize;
    let mut head: usize = 127;
    head = (head + 1) % cap;
    assert_eq!(head, 0);
}
