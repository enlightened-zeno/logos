//! LogOS init process (PID 1).
//!
//! Mounts filesystems, spawns the shell, reaps zombies, respawns on shell exit.

#![no_std]
#![no_main]

use liblogos::syscall;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Write startup message
    syscall::write(1, b"LogOS init: starting\n");

    // PID 1 main loop: spawn shell, wait, respawn
    loop {
        syscall::write(1, b"init: spawning shell\n");

        // In a full implementation:
        // let pid = syscall::fork();
        // if pid == 0 {
        //     syscall::execve("/bin/lsh", &[], &[]);
        //     syscall::exit(1);
        // }
        // syscall::wait4(pid, ...);

        // For now, just halt
        syscall::exit(0);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    syscall::write(2, b"init: PANIC\n");
    syscall::exit(1);
}
