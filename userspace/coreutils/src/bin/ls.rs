#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Would use getdents64 syscall in full implementation
    syscall::write(1, b"ls: not yet implemented for userspace\n");
    syscall::exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
