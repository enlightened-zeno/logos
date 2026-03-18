extern crate alloc;

use crate::drivers::pci::PciDevice;
use crate::memory::addr::{PhysAddr, VirtAddr};
use crate::memory::paging::PageFlags;
use crate::memory::vmm::Vmm;
use crate::sync::SpinLock;

/// AHCI HBA Memory Registers (MMIO).
#[repr(C)]
struct HbaMemory {
    cap: u32,
    ghc: u32,
    is: u32,
    pi: u32, // Ports implemented (bitmask)
    vs: u32, // Version
    ccc_ctl: u32,
    ccc_ports: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: u32,
    bohc: u32,
    _reserved: [u8; 0xA0 - 0x2C],
    vendor: [u8; 0x100 - 0xA0],
    ports: [HbaPort; 32],
}

/// AHCI port registers.
#[repr(C)]
struct HbaPort {
    clb: u32,  // Command list base address (low)
    clbu: u32, // Command list base address (high)
    fb: u32,   // FIS base address (low)
    fbu: u32,  // FIS base address (high)
    is: u32,   // Interrupt status
    ie: u32,   // Interrupt enable
    cmd: u32,  // Command and status
    _reserved0: u32,
    tfd: u32,  // Task file data
    sig: u32,  // Signature
    ssts: u32, // SATA status
    sctl: u32, // SATA control
    serr: u32, // SATA error
    sact: u32, // SATA active
    ci: u32,   // Command issue
    sntf: u32, // SATA notification
    fbs: u32,  // FIS-based switching control
    _reserved1: [u32; 11],
    vendor: [u32; 4],
}

/// SATA device signature for ATA drives.
const SATA_SIG_ATA: u32 = 0x00000101;
/// Port command: start
const HBA_CMD_ST: u32 = 1 << 0;
/// Port command: FIS receive enable
const HBA_CMD_FRE: u32 = 1 << 4;
/// Port command: FIS receive running
const HBA_CMD_FR: u32 = 1 << 14;
/// Port command: command list running
const HBA_CMD_CR: u32 = 1 << 15;

/// AHCI PCI class/subclass.
pub const AHCI_CLASS: u8 = 0x01;
pub const AHCI_SUBCLASS: u8 = 0x06;

struct AhciState {
    hba: *mut HbaMemory,
    port_count: u32,
    port_mask: u32,
}

// SAFETY: Protected by SpinLock.
unsafe impl Send for AhciState {}

static AHCI: SpinLock<Option<AhciState>> = SpinLock::new(None);

/// Initialize AHCI controller from a PCI device.
///
/// # Safety
/// PCI device must be a valid AHCI controller.
pub unsafe fn init(pci: &PciDevice, hhdm_offset: u64) -> Result<(), &'static str> {
    // BAR5 contains the AHCI MMIO base (ABAR)
    let bar5 = pci.bar[5];
    if bar5 & 1 != 0 {
        return Err("AHCI: BAR5 is I/O space, expected MMIO");
    }

    let ahci_phys = (bar5 & 0xFFFFF000) as u64;
    let ahci_virt = ahci_phys + hhdm_offset;

    // Map the AHCI MMIO region (typically 1 page)
    let vmm = Vmm::get();
    let _ = vmm.map_page(
        VirtAddr::new_canonicalize(ahci_virt),
        crate::memory::addr::PhysFrame::containing_address(PhysAddr::new(ahci_phys)),
        PageFlags::WRITABLE | PageFlags::NO_EXECUTE | PageFlags::NO_CACHE,
    );

    let hba = ahci_virt as *mut HbaMemory;

    // SAFETY: MMIO region is mapped and valid.
    let pi = unsafe { (*hba).pi };
    let vs = unsafe { (*hba).vs };
    let cap = unsafe { (*hba).cap };
    let port_count = (cap & 0x1F) + 1;

    crate::serial_println!(
        "AHCI: version {}.{}, {} ports, implemented mask={:#x}",
        vs >> 16,
        vs & 0xFFFF,
        port_count,
        pi
    );

    // Scan implemented ports for SATA devices
    for i in 0..32 {
        if pi & (1 << i) == 0 {
            continue;
        }

        // SAFETY: Port index is within the implemented mask.
        let port = unsafe { &(*hba).ports[i] };
        let ssts = unsafe { core::ptr::read_volatile(&port.ssts) };
        let det = ssts & 0xF;
        let ipm = (ssts >> 8) & 0xF;

        if det == 3 && ipm == 1 {
            // Device present and active
            let sig = unsafe { core::ptr::read_volatile(&port.sig) };
            let device_type = if sig == SATA_SIG_ATA { "SATA" } else { "other" };
            crate::serial_println!("  Port {}: {} device (sig={:#x})", i, device_type, sig);
        }
    }

    *AHCI.lock() = Some(AhciState {
        hba,
        port_count,
        port_mask: pi,
    });

    Ok(())
}

/// Check if AHCI is initialized.
pub fn is_available() -> bool {
    AHCI.lock().is_some()
}
