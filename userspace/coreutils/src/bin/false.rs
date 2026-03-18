#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::exit(1);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
