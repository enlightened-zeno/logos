use core::fmt;

/// Write to stdout (fd 1).
pub fn print(s: &str) {
    crate::syscall::write(1, s.as_bytes());
}

/// Print macro for userspace programs.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::_print(format_args!($($arg)*))
    };
}

/// Println macro.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    let mut w = StdoutWriter;
    let _ = w.write_fmt(args);
}

struct StdoutWriter;

impl fmt::Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::syscall::write(1, s.as_bytes());
        Ok(())
    }
}
