extern crate alloc;

use crate::arch::x86_64::io;
use alloc::vec::Vec;

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// A discovered PCI device.
#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: u8,
    pub bar: [u32; 6],
    pub interrupt_line: u8,
}

/// Read a 32-bit value from PCI config space.
pub fn config_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address: u32 = 0x8000_0000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);

    io::outl(PCI_CONFIG_ADDR, address);
    io::inl(PCI_CONFIG_DATA)
}

/// Write a 32-bit value to PCI config space.
pub fn config_write32(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address: u32 = 0x8000_0000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);

    io::outl(PCI_CONFIG_ADDR, address);
    io::outl(PCI_CONFIG_DATA, value);
}

/// Read a 16-bit value from PCI config space.
pub fn config_read16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let val = config_read32(bus, device, function, offset & 0xFC);
    ((val >> ((offset & 2) * 8)) & 0xFFFF) as u16
}

/// Enumerate all PCI devices on bus 0.
pub fn enumerate() -> Vec<PciDevice> {
    let mut devices = Vec::new();

    for bus in 0..=255u16 {
        for device in 0..32u8 {
            let vendor = config_read16(bus as u8, device, 0, 0x00);
            if vendor == 0xFFFF {
                continue;
            }

            let header_type = (config_read32(bus as u8, device, 0, 0x0C) >> 16) as u8 & 0x7F;
            let max_func = if header_type & 0x80 != 0 { 8 } else { 1 };

            for function in 0..max_func {
                let vendor_id = config_read16(bus as u8, device, function, 0x00);
                if vendor_id == 0xFFFF {
                    continue;
                }

                let device_id = config_read16(bus as u8, device, function, 0x02);
                let class_code = config_read32(bus as u8, device, function, 0x08);
                let class = (class_code >> 24) as u8;
                let subclass = (class_code >> 16) as u8;
                let prog_if = (class_code >> 8) as u8;

                let mut bar = [0u32; 6];
                for (i, b) in bar.iter_mut().enumerate() {
                    *b = config_read32(bus as u8, device, function, 0x10 + (i as u8) * 4);
                }

                let interrupt_line = config_read32(bus as u8, device, function, 0x3C) as u8;

                devices.push(PciDevice {
                    bus: bus as u8,
                    device,
                    function,
                    vendor_id,
                    device_id,
                    class,
                    subclass,
                    prog_if,
                    header_type,
                    bar,
                    interrupt_line,
                });
            }
        }
        // Only scan bus 0 for speed (most QEMU devices are here)
        if bus == 0 {
            break;
        }
    }

    devices
}

/// Find a PCI device by vendor and device ID.
pub fn find_device(devices: &[PciDevice], vendor: u16, device_id: u16) -> Option<&PciDevice> {
    devices
        .iter()
        .find(|d| d.vendor_id == vendor && d.device_id == device_id)
}

/// VirtIO vendor ID.
pub const VIRTIO_VENDOR: u16 = 0x1AF4;
/// VirtIO block device (legacy device ID range: 0x1000-0x103F, block = 0x1001)
pub const VIRTIO_BLOCK_DEVICE: u16 = 0x1001;
