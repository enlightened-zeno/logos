use crate::arch::x86_64::apic;
use crate::arch::x86_64::idt::InterruptFrame;

/// APIC timer interrupt handler.
pub extern "x86-interrupt" fn timer_handler(_frame: InterruptFrame) {
    apic::tick();
    crate::timer::tick();
    crate::sched::timer_tick();
    apic::eoi();
}

/// PS/2 keyboard interrupt handler (IRQ1 → vector 0x21).
pub extern "x86-interrupt" fn keyboard_handler(_frame: InterruptFrame) {
    crate::drivers::keyboard::handle_scancode();
    apic::eoi();
}
