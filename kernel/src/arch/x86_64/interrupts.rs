use crate::arch::x86_64::apic;
use crate::arch::x86_64::idt::InterruptFrame;

/// APIC timer interrupt handler.
pub extern "x86-interrupt" fn timer_handler(_frame: InterruptFrame) {
    apic::tick();
    crate::sched::timer_tick();
    apic::eoi();
}
