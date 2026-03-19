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

// === AHCI Command Structures ===

/// FIS type: Register Host to Device
const FIS_TYPE_REG_H2D: u8 = 0x27;
/// ATA command: READ DMA EXT (48-bit LBA)
const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
/// ATA command: WRITE DMA EXT (48-bit LBA)
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
/// ATA command: IDENTIFY DEVICE
const ATA_CMD_IDENTIFY: u8 = 0xEC;

/// AHCI Command Header (32 bytes each, 32 per port).
#[repr(C)]
#[derive(Clone, Copy)]
struct CommandHeader {
    flags: u16, // CFL (bits 0-4), ATAPI, Write, Prefetch, etc.
    prdtl: u16, // Physical Region Descriptor Table Length
    prdbc: u32, // PRD Byte Count (transferred)
    ctba: u32,  // Command Table Base Address (low)
    ctbau: u32, // Command Table Base Address (high)
    _reserved: [u32; 4],
}

/// Physical Region Descriptor Table entry.
#[repr(C)]
#[derive(Clone, Copy)]
struct PrdtEntry {
    dba: u32,  // Data Base Address (low)
    dbau: u32, // Data Base Address (high)
    _reserved: u32,
    dbc: u32, // Byte Count (bit 0 must be 1, max 4MB per entry), bit 31 = interrupt
}

/// Register Host to Device FIS (20 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
struct FisRegH2D {
    fis_type: u8, // FIS_TYPE_REG_H2D
    flags: u8,    // bit 7 = command (vs control)
    command: u8,  // ATA command
    featurel: u8, // Feature low
    lba0: u8,     // LBA 7:0
    lba1: u8,     // LBA 15:8
    lba2: u8,     // LBA 23:16
    device: u8,   // Device (bit 6 = LBA mode)
    lba3: u8,     // LBA 31:24
    lba4: u8,     // LBA 39:32
    lba5: u8,     // LBA 47:40
    featureh: u8, // Feature high
    countl: u8,   // Sector count low
    counth: u8,   // Sector count high
    icc: u8,
    control: u8,
    _reserved: [u8; 4],
}

/// Command Table (CFIS + PRDT).
/// Must be 128-byte aligned, contains a CFIS area and PRDT entries.
#[repr(C, align(128))]
struct CommandTable {
    cfis: [u8; 64], // Command FIS
    acmd: [u8; 16], // ATAPI command (unused for SATA)
    _reserved: [u8; 48],
    prdt: [PrdtEntry; 1], // At least 1 PRDT entry
}

/// Read sectors from the first SATA port.
///
/// # Safety
/// AHCI must be initialized and the port must be started.
pub fn read_sectors(lba: u64, sector_count: u16, buf: &mut [u8]) -> Result<(), &'static str> {
    if crate::fault::should_fail(crate::fault::InjectionPoint::DiskRead) {
        return Err("Injected AHCI read failure");
    }

    let guard = AHCI.lock();
    let state = guard.as_ref().ok_or("AHCI not initialized")?;

    // Find the first active SATA port
    let port_num = find_sata_port(state).ok_or("No SATA device found")?;

    // SAFETY: Port is within the implemented mask, MMIO region is mapped.
    unsafe {
        let port = &mut (*state.hba).ports[port_num];
        issue_ata_command(
            port,
            ATA_CMD_READ_DMA_EXT,
            lba,
            sector_count,
            buf.as_mut_ptr() as u64,
            buf.len(),
            false,
        )
    }
}

/// Write sectors to the first SATA port.
pub fn write_sectors(lba: u64, sector_count: u16, buf: &[u8]) -> Result<(), &'static str> {
    if crate::fault::should_fail(crate::fault::InjectionPoint::DiskWrite) {
        return Err("Injected AHCI write failure");
    }

    let guard = AHCI.lock();
    let state = guard.as_ref().ok_or("AHCI not initialized")?;

    let port_num = find_sata_port(state).ok_or("No SATA device found")?;

    // SAFETY: Port is within the implemented mask.
    unsafe {
        let port = &mut (*state.hba).ports[port_num];
        issue_ata_command(
            port,
            ATA_CMD_WRITE_DMA_EXT,
            lba,
            sector_count,
            buf.as_ptr() as u64,
            buf.len(),
            true,
        )
    }
}

fn find_sata_port(state: &AhciState) -> Option<usize> {
    for i in 0..32 {
        if state.port_mask & (1 << i) == 0 {
            continue;
        }
        // SAFETY: Port is within the implemented mask.
        let port = unsafe { &(*state.hba).ports[i] };
        let ssts = unsafe { core::ptr::read_volatile(&port.ssts) };
        let det = ssts & 0xF;
        let ipm = (ssts >> 8) & 0xF;
        if det == 3 && ipm == 1 {
            let sig = unsafe { core::ptr::read_volatile(&port.sig) };
            if sig == SATA_SIG_ATA {
                return Some(i);
            }
        }
    }
    None
}

/// Issue an ATA command to a port using command slot 0.
///
/// # Safety
/// Port must be valid and started. Buffer address must be DMA-accessible.
unsafe fn issue_ata_command(
    port: &mut HbaPort,
    command: u8,
    lba: u64,
    sector_count: u16,
    buf_phys: u64,
    buf_len: usize,
    is_write: bool,
) -> Result<(), &'static str> {
    // Wait for port to not be busy
    let mut spin = 0u32;
    while unsafe { core::ptr::read_volatile(&port.tfd) } & 0x88 != 0 {
        // BSY=1 or DRQ=1
        spin += 1;
        if spin > 1_000_000 {
            return Err("AHCI: port busy timeout");
        }
        core::hint::spin_loop();
    }

    // Get command list base
    let clb = unsafe {
        let lo = core::ptr::read_volatile(&port.clb) as u64;
        let hi = core::ptr::read_volatile(&port.clbu) as u64;
        lo | (hi << 32)
    };

    if clb == 0 {
        return Err("AHCI: command list not configured");
    }

    let pmm = crate::memory::pmm::Pmm::get();
    let hhdm = pmm.hhdm_offset();
    let cmd_header = (clb + hhdm) as *mut CommandHeader;

    // Set up command header (slot 0)
    // SAFETY: CLB points to valid command list memory.
    unsafe {
        let hdr = &mut *cmd_header;
        let cfl = (core::mem::size_of::<FisRegH2D>() / 4) as u16; // FIS length in DWORDs
        hdr.flags = cfl;
        if is_write {
            hdr.flags |= 1 << 6; // Write bit
        }
        hdr.prdtl = 1; // 1 PRDT entry
        hdr.prdbc = 0;

        // Allocate a command table (simplified: use a fixed address)
        // In a full implementation, we'd pre-allocate per-slot command tables.
        let ct_frame = pmm.alloc().ok_or("AHCI: OOM for command table")?;
        let ct_phys = ct_frame.start_address().as_u64();
        hdr.ctba = ct_phys as u32;
        hdr.ctbau = (ct_phys >> 32) as u32;

        // Set up command table
        let ct = (ct_phys + hhdm) as *mut CommandTable;
        core::ptr::write_bytes(ct, 0, 1); // Zero the command table

        // Build Register H2D FIS
        let fis = (*ct).cfis.as_mut_ptr() as *mut FisRegH2D;
        (*fis).fis_type = FIS_TYPE_REG_H2D;
        (*fis).flags = 0x80; // Command bit
        (*fis).command = command;
        (*fis).device = 1 << 6; // LBA mode
        (*fis).lba0 = (lba & 0xFF) as u8;
        (*fis).lba1 = ((lba >> 8) & 0xFF) as u8;
        (*fis).lba2 = ((lba >> 16) & 0xFF) as u8;
        (*fis).lba3 = ((lba >> 24) & 0xFF) as u8;
        (*fis).lba4 = ((lba >> 32) & 0xFF) as u8;
        (*fis).lba5 = ((lba >> 40) & 0xFF) as u8;
        (*fis).countl = (sector_count & 0xFF) as u8;
        (*fis).counth = ((sector_count >> 8) & 0xFF) as u8;

        // Set up PRDT entry
        (*ct).prdt[0].dba = buf_phys as u32;
        (*ct).prdt[0].dbau = (buf_phys >> 32) as u32;
        (*ct).prdt[0].dbc = (buf_len as u32 - 1) | (1 << 31); // Byte count - 1, interrupt on completion

        // Issue command (set bit 0 in CI register)
        core::ptr::write_volatile(&mut port.ci, 1);

        // Poll for completion
        let mut timeout = 0u32;
        loop {
            let ci = core::ptr::read_volatile(&port.ci);
            if ci & 1 == 0 {
                break; // Command completed
            }
            let is_val = core::ptr::read_volatile(&port.is);
            if is_val & (1 << 30) != 0 {
                // Task file error
                pmm.dealloc(ct_frame);
                return Err("AHCI: task file error");
            }
            timeout += 1;
            if timeout > 10_000_000 {
                pmm.dealloc(ct_frame);
                return Err("AHCI: command timeout");
            }
            core::hint::spin_loop();
        }

        // Free command table
        pmm.dealloc(ct_frame);
    }

    Ok(())
}
