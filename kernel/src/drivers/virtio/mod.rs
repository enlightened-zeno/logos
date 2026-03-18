pub mod block;

use crate::arch::x86_64::io;

/// VirtIO legacy PCI device status bits.
pub const STATUS_ACKNOWLEDGE: u8 = 1;
pub const STATUS_DRIVER: u8 = 2;
pub const STATUS_DRIVER_OK: u8 = 4;
pub const STATUS_FEATURES_OK: u8 = 8;

/// VirtIO legacy PCI register offsets (from BAR0 I/O base).
pub const REG_DEVICE_FEATURES: u16 = 0;
pub const REG_GUEST_FEATURES: u16 = 4;
pub const REG_QUEUE_ADDRESS: u16 = 8;
pub const REG_QUEUE_SIZE: u16 = 12;
pub const REG_QUEUE_SELECT: u16 = 14;
pub const REG_QUEUE_NOTIFY: u16 = 16;
pub const REG_DEVICE_STATUS: u16 = 18;
pub const REG_ISR_STATUS: u16 = 19;

/// Read a VirtIO register (32-bit).
pub fn read32(base: u16, offset: u16) -> u32 {
    io::inl(base + offset)
}

/// Write a VirtIO register (32-bit).
pub fn write32(base: u16, offset: u16, val: u32) {
    io::outl(base + offset, val);
}

/// Read a VirtIO register (16-bit).
pub fn read16(base: u16, offset: u16) -> u16 {
    io::inw(base + offset)
}

/// Write a VirtIO register (16-bit).
pub fn write16(base: u16, offset: u16, val: u16) {
    io::outw(base + offset, val);
}

/// Read a VirtIO register (8-bit).
pub fn read8(base: u16, offset: u16) -> u8 {
    io::inb(base + offset)
}

/// Write a VirtIO register (8-bit).
pub fn write8(base: u16, offset: u16, val: u8) {
    io::outb(base + offset, val);
}
