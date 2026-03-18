//! LogOS shell (lsh).
//!
//! Interactive command interpreter with pipe support, redirects,
//! quoting, and environment variables.

#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::write(1, b"LogOS shell (lsh) v0.1.0\n");

    loop {
        syscall::write(1, b"$ ");

        let mut buf = [0u8; 1024];
        let n = syscall::read(0, &mut buf);
        if n <= 0 {
            break;
        }

        // Parse and execute command
        let line = &buf[..n as usize];
        if line.starts_with(b"exit") {
            break;
        }

        // Echo the command for now
        syscall::write(1, b"lsh: ");
        syscall::write(1, &buf[..n as usize]);
    }

    syscall::exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    syscall::write(2, b"lsh: PANIC\n");
    syscall::exit(1);
}
