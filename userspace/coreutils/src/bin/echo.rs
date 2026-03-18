#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In a full implementation, we'd parse argc/argv from the stack.
    // For now, just echo "hello" as a proof of concept.
    syscall::write(1, b"hello\n");
    syscall::exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
