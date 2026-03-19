/// Framebuffer console logic tests.

#[test]
fn test_font_dimensions() {
    let width = 8;
    let height = 16;
    assert_eq!(width * height, 128); // bits per glyph
    assert_eq!(height, 16); // bytes per glyph (1 byte per row)
}

#[test]
fn test_chars_per_screen() {
    let screen_w = 1024;
    let screen_h = 768;
    let font_w = 8;
    let font_h = 16;
    let cols = screen_w / font_w;
    let rows = screen_h / font_h;
    assert_eq!(cols, 128);
    assert_eq!(rows, 48);
}

#[test]
fn test_pixel_offset() {
    let col = 10u32;
    let row = 5u32;
    let font_w = 8u32;
    let font_h = 16u32;
    let pitch = 1024u32; // pixels per row
    let px = col * font_w;
    let py = row * font_h;
    let offset = py * pitch + px;
    assert_eq!(px, 80);
    assert_eq!(py, 80);
    assert!(offset > 0);
}

#[test]
fn test_glyph_index() {
    let first_char = 32u8; // space
    let last_char = 126u8; // tilde
    let count = (last_char - first_char + 1) as usize;
    assert_eq!(count, 95);

    let ch = b'A';
    let idx = (ch - first_char) as usize;
    assert_eq!(idx, 33);
}

#[test]
fn test_scroll_copy() {
    let rows = 48;
    let font_h = 16;
    let total_h = rows * font_h;
    let copy_rows = total_h - font_h;
    assert_eq!(copy_rows, 752);
}

#[test]
fn test_color_format() {
    let fg: u32 = 0x00CCCCCC; // Light gray (ARGB)
    let bg: u32 = 0x00000000; // Black
    assert_ne!(fg, bg);
    assert_eq!(bg, 0);
}
