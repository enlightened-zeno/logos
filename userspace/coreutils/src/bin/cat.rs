#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Read from stdin, write to stdout
    let mut buf = [0u8; 4096];
    loop {
        let n = syscall::read(0, &mut buf);
        if n <= 0 {
            break;
        }
        syscall::write(1, &buf[..n as usize]);
    }
    syscall::exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
