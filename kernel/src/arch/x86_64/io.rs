#![allow(dead_code)]

/// Write a byte to an I/O port.
#[inline(always)]
pub fn outb(port: u16, value: u8) {
    // SAFETY: Writing to an I/O port is safe when the port address is valid.
    // Callers must ensure the port is a legitimate hardware register.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack),
        );
    }
}

/// Read a byte from an I/O port.
#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let value: u8;
    // SAFETY: Reading from an I/O port is safe when the port address is valid.
    // Callers must ensure the port is a legitimate hardware register.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack),
        );
    }
    value
}

/// Write a 16-bit word to an I/O port.
#[inline(always)]
pub fn outw(port: u16, value: u16) {
    // SAFETY: Writing to an I/O port is safe when the port address is valid.
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(nomem, nostack),
        );
    }
}

/// Read a 16-bit word from an I/O port.
#[inline(always)]
pub fn inw(port: u16) -> u16 {
    let value: u16;
    // SAFETY: Reading from an I/O port is safe when the port address is valid.
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            out("ax") value,
            in("dx") port,
            options(nomem, nostack),
        );
    }
    value
}

/// Write a 32-bit doubleword to an I/O port.
#[inline(always)]
pub fn outl(port: u16, value: u32) {
    // SAFETY: Writing to an I/O port is safe when the port address is valid.
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack),
        );
    }
}

/// Read a 32-bit doubleword from an I/O port.
#[inline(always)]
pub fn inl(port: u16) -> u32 {
    let value: u32;
    // SAFETY: Reading from an I/O port is safe when the port address is valid.
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            out("eax") value,
            in("dx") port,
            options(nomem, nostack),
        );
    }
    value
}
