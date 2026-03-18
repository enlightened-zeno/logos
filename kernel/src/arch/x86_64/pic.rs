use crate::arch::x86_64::io;

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// Disable the legacy 8259 PIC by masking all IRQs.
///
/// Must be called before enabling the APIC to prevent spurious interrupts.
///
/// # Safety
/// Must be called during boot before the APIC is initialized.
pub unsafe fn disable() {
    // Remap PIC to vectors 0x20-0x2F to avoid conflicts with CPU exceptions
    io::outb(PIC1_CMD, 0x11); // ICW1: init + ICW4 needed
    io::outb(PIC2_CMD, 0x11);
    io::outb(PIC1_DATA, 0x20); // ICW2: PIC1 vector offset
    io::outb(PIC2_DATA, 0x28); // ICW2: PIC2 vector offset
    io::outb(PIC1_DATA, 0x04); // ICW3: PIC2 at IRQ2
    io::outb(PIC2_DATA, 0x02); // ICW3: cascade identity
    io::outb(PIC1_DATA, 0x01); // ICW4: 8086 mode
    io::outb(PIC2_DATA, 0x01);

    // Mask all IRQs
    io::outb(PIC1_DATA, 0xFF);
    io::outb(PIC2_DATA, 0xFF);
}
