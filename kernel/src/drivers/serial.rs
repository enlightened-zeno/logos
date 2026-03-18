use crate::arch::x86_64::io::{inb, outb};
use core::fmt;
use spin::Mutex;

/// COM1 base I/O port address.
const COM1: u16 = 0x3F8;

/// 16550 UART serial port driver.
pub struct Serial {
    port: u16,
}

impl Serial {
    /// Create a new serial port driver for the given base I/O address.
    const fn new(port: u16) -> Self {
        Self { port }
    }

    /// Initialize the serial port: 115200 baud, 8N1, FIFO enabled.
    pub fn init(&self) {
        let port = self.port;

        // Disable all interrupts
        outb(port + 1, 0x00);

        // Enable DLAB (Divisor Latch Access Bit) to set baud rate
        outb(port + 3, 0x80);

        // Set divisor to 1 (115200 baud) - low byte
        outb(port, 0x01);
        // High byte
        outb(port + 1, 0x00);

        // 8 bits, no parity, one stop bit (8N1), disable DLAB
        outb(port + 3, 0x03);

        // Enable FIFO, clear TX/RX queues, 14-byte threshold
        outb(port + 2, 0xC7);

        // IRQs enabled, RTS/DSR set
        outb(port + 4, 0x0B);

        // Set loopback mode for self-test
        outb(port + 4, 0x1E);

        // Send test byte
        outb(port, 0xAE);

        // Check if we received the test byte back
        if inb(port) != 0xAE {
            // Serial port failed self-test — nothing we can do, just continue.
            // Output may not work but we won't hang.
        }

        // Disable loopback, set normal operation mode
        outb(port + 4, 0x0F);
    }

    /// Wait until the transmit buffer is empty, then send a byte.
    fn write_byte(&self, byte: u8) {
        // Wait for the transmit holding register to be empty
        while inb(self.port + 5) & 0x20 == 0 {
            core::hint::spin_loop();
        }
        outb(self.port, byte);
    }

    /// Write a string to the serial port, converting '\n' to '\r\n'.
    pub fn write_str_serial(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_serial(s);
        Ok(())
    }
}

/// Global serial port instance, protected by a spinlock.
pub static SERIAL1: Mutex<Serial> = Mutex::new(Serial::new(COM1));

/// Initialize the primary serial port (COM1).
pub fn init() {
    SERIAL1.lock().init();
}

/// Write a single byte to the serial port.
pub fn write_byte(byte: u8) {
    SERIAL1.lock().write_byte(byte);
}

/// Print formatted text to the serial port.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_serial_print(format_args!($($arg)*))
    };
}

/// Print formatted text to the serial port, followed by a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

/// Internal print function — not for direct use. Use `serial_print!` macro.
#[doc(hidden)]
pub fn _serial_print(args: fmt::Arguments) {
    use fmt::Write;

    // Write to serial
    SERIAL1.lock().write_fmt(args).unwrap();

    // Capture to kernel log ring buffer
    let mut log_buf = LogCapture;
    let _ = log_buf.write_fmt(args);

    // Dual output to framebuffer console
    let _ = crate::drivers::framebuffer::FbWriter.write_fmt(args);
}

/// Helper to capture formatted output into the log buffer.
struct LogCapture;

impl fmt::Write for LogCapture {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::log::append(s.as_bytes());
        Ok(())
    }
}
