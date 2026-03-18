extern crate alloc;

use crate::sync::SpinLock;
use core::fmt;

/// 8x16 bitmap font (ASCII 32-126, each char is 16 bytes = 16 rows of 8 pixels).
/// This is a minimal built-in font covering printable ASCII.
static FONT_8X16: &[u8] = include_bytes!("font8x16.raw");

const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 16;
const FONT_FIRST_CHAR: u8 = 32;
const FONT_LAST_CHAR: u8 = 126;

/// Framebuffer console state.
struct FbConsole {
    /// Framebuffer base address (virtual, via HHDM).
    base: *mut u32,
    /// Pixels per row (may differ from width due to padding).
    pitch: u32,
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// Current cursor position (in character cells).
    col: u32,
    row: u32,
    /// Max characters per row/column.
    cols: u32,
    rows: u32,
    /// Colors (ARGB 32-bit).
    fg: u32,
    bg: u32,
}

// SAFETY: Protected by SpinLock.
unsafe impl Send for FbConsole {}

static FB: SpinLock<Option<FbConsole>> = SpinLock::new(None);
static FB_ACTIVE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Enable framebuffer output. Call after boot init is complete.
pub fn activate() {
    FB_ACTIVE.store(true, core::sync::atomic::Ordering::Release);
}

/// Initialize the framebuffer console from Limine framebuffer info.
///
/// # Safety
/// `base` must be a valid writable framebuffer address. `pitch` is in bytes.
pub unsafe fn init(base: u64, width: u32, height: u32, pitch: u32, bpp: u16) {
    if bpp != 32 {
        crate::serial_println!("FB: unsupported bpp={}, need 32", bpp);
        return;
    }

    let cols = width / FONT_WIDTH;
    let rows = height / FONT_HEIGHT;

    let fb = FbConsole {
        base: base as *mut u32,
        pitch: pitch / 4, // Convert byte pitch to pixel pitch
        width,
        height,
        col: 0,
        row: 0,
        cols,
        rows,
        fg: 0x00CCCCCC, // Light gray
        bg: 0x00000000, // Black
    };

    // Skip clearing — Limine already provides a black framebuffer.
    // clear_screen is available via the `clear` shell command.

    *FB.lock() = Some(fb);

    crate::serial_println!("FB: {}x{} ({}x{} chars)", width, height, cols, rows);
}

/// Write a string to the framebuffer console.
pub fn write_str(s: &str) {
    if !FB_ACTIVE.load(core::sync::atomic::Ordering::Acquire) {
        return;
    }
    let mut guard = FB.lock();
    let fb = match guard.as_mut() {
        Some(fb) => fb,
        None => return,
    };

    for byte in s.bytes() {
        put_char(fb, byte);
    }
}

/// Write a single byte to the framebuffer console.
#[allow(dead_code)]
pub fn write_byte_fb(byte: u8) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        put_char(fb, byte);
    }
}

fn put_char(fb: &mut FbConsole, byte: u8) {
    match byte {
        b'\n' => {
            fb.col = 0;
            fb.row += 1;
            if fb.row >= fb.rows {
                scroll_up(fb);
                fb.row = fb.rows - 1;
            }
        }
        b'\r' => {
            fb.col = 0;
        }
        b'\t' => {
            let next = (fb.col + 8) & !7;
            fb.col = next.min(fb.cols - 1);
        }
        // Backspace
        0x08 => {
            if fb.col > 0 {
                fb.col -= 1;
                draw_glyph(fb, fb.col, fb.row, b' ');
            }
        }
        // ESC sequence: minimal support for \x1b[2J (clear) and \x1b[H (home)
        0x1b => {
            // Skip — we'd need a state machine for full ANSI parsing.
            // For now, just ignore escape chars.
        }
        byte => {
            draw_glyph(fb, fb.col, fb.row, byte);
            fb.col += 1;
            if fb.col >= fb.cols {
                fb.col = 0;
                fb.row += 1;
                if fb.row >= fb.rows {
                    scroll_up(fb);
                    fb.row = fb.rows - 1;
                }
            }
        }
    }
}

fn draw_glyph(fb: &FbConsole, col: u32, row: u32, ch: u8) {
    let glyph_idx = if (FONT_FIRST_CHAR..=FONT_LAST_CHAR).contains(&ch) {
        (ch - FONT_FIRST_CHAR) as usize
    } else {
        0 // Space for non-printable
    };

    let glyph = &FONT_8X16[glyph_idx * FONT_HEIGHT as usize..][..FONT_HEIGHT as usize];
    let px = col * FONT_WIDTH;
    let py = row * FONT_HEIGHT;

    for (y, &glyph_row) in glyph.iter().enumerate() {
        for x in 0..FONT_WIDTH {
            let pixel = if glyph_row & (0x80 >> x) != 0 {
                fb.fg
            } else {
                fb.bg
            };
            let offset = (py + y as u32) * fb.pitch + (px + x);
            // SAFETY: We verified dimensions during init. This pixel is within bounds.
            unsafe {
                fb.base.add(offset as usize).write_volatile(pixel);
            }
        }
    }
}

fn scroll_up(fb: &mut FbConsole) {
    // Copy rows 1..n to 0..n-1
    for y in 0..(fb.height - FONT_HEIGHT) {
        let src_offset = (y + FONT_HEIGHT) * fb.pitch;
        let dst_offset = y * fb.pitch;
        for x in 0..fb.width {
            // SAFETY: Source and destination are within framebuffer bounds.
            unsafe {
                let pixel = fb.base.add((src_offset + x) as usize).read_volatile();
                fb.base.add((dst_offset + x) as usize).write_volatile(pixel);
            }
        }
    }

    // Clear the last row
    let last_row_start = (fb.height - FONT_HEIGHT) * fb.pitch;
    for y in 0..FONT_HEIGHT {
        for x in 0..fb.width {
            // SAFETY: Within framebuffer bounds.
            unsafe {
                fb.base
                    .add((last_row_start + y * fb.pitch + x) as usize)
                    .write_volatile(fb.bg);
            }
        }
    }
}

#[allow(dead_code)]
fn clear_screen(fb: &mut FbConsole) {
    for y in 0..fb.height {
        for x in 0..fb.width {
            // SAFETY: Within framebuffer bounds.
            unsafe {
                fb.base
                    .add((y * fb.pitch + x) as usize)
                    .write_volatile(fb.bg);
            }
        }
    }
    fb.col = 0;
    fb.row = 0;
}

/// fmt::Write implementation for dual serial+framebuffer output.
pub struct FbWriter;

impl fmt::Write for FbWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
