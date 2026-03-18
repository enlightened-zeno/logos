#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::write(1, b"LogOS 0.1.0 x86_64\n");
    syscall::exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
