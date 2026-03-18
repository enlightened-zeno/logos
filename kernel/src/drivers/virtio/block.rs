extern crate alloc;

use crate::drivers::pci::PciDevice;
use crate::drivers::virtio;
use crate::memory::addr::PAGE_SIZE;
use crate::memory::pmm::Pmm;
use crate::sync::SpinLock;
use core::sync::atomic::{fence, Ordering};

/// VirtIO block device sector size.
pub const SECTOR_SIZE: u64 = 512;

/// VirtIO block request types.
const VIRTIO_BLK_T_IN: u32 = 0; // Read
const VIRTIO_BLK_T_OUT: u32 = 1; // Write

/// VirtIO block config offset (after general registers, at offset 20 for legacy).
const CFG_CAPACITY: u16 = 20;

/// Virtqueue descriptor flags.
const VRING_DESC_F_NEXT: u16 = 1;
const VRING_DESC_F_WRITE: u16 = 2;

/// Virtqueue descriptor.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VringDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// Virtqueue available ring.
#[repr(C)]
struct VringAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 128],
}

/// Virtqueue used ring entry.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VringUsedElem {
    id: u32,
    len: u32,
}

/// Virtqueue used ring.
#[repr(C)]
struct VringUsed {
    flags: u16,
    idx: u16,
    ring: [VringUsedElem; 128],
}

/// VirtIO block request header.
#[repr(C)]
struct VirtioBlkReqHeader {
    req_type: u32,
    _reserved: u32,
    sector: u64,
}

/// VirtIO block device driver state.
struct VirtioBlock {
    io_base: u16,
    capacity_sectors: u64,
    /// Virtqueue descriptors (physical address).
    desc_phys: u64,
    desc: *mut VringDesc,
    /// Available ring (physical address).
    avail_phys: u64,
    avail: *mut VringAvail,
    /// Used ring (physical address).
    used_phys: u64,
    used: *mut VringUsed,
    /// Queue size.
    queue_size: u16,
    /// Next descriptor index to allocate.
    next_desc: u16,
    /// Last seen used index.
    last_used_idx: u16,
    /// HHDM offset for phys-to-virt.
    hhdm_offset: u64,
}

// SAFETY: Protected by SpinLock.
unsafe impl Send for VirtioBlock {}

static BLOCK_DEV: SpinLock<Option<VirtioBlock>> = SpinLock::new(None);

/// Initialize the VirtIO block device from a PCI device.
///
/// # Safety
/// The PCI device must be a valid VirtIO block device.
pub unsafe fn init(pci: &PciDevice, hhdm_offset: u64) -> Result<(), &'static str> {
    // BAR0 should be I/O space (bit 0 set)
    let bar0 = pci.bar[0];
    if bar0 & 1 == 0 {
        return Err("VirtIO block: BAR0 is not I/O space");
    }
    let io_base = (bar0 & 0xFFFC) as u16;

    // Reset device
    virtio::write8(io_base, virtio::REG_DEVICE_STATUS, 0);

    // Acknowledge + driver
    virtio::write8(
        io_base,
        virtio::REG_DEVICE_STATUS,
        virtio::STATUS_ACKNOWLEDGE,
    );
    virtio::write8(
        io_base,
        virtio::REG_DEVICE_STATUS,
        virtio::STATUS_ACKNOWLEDGE | virtio::STATUS_DRIVER,
    );

    // Read features, accept none (simplest path)
    let _features = virtio::read32(io_base, virtio::REG_DEVICE_FEATURES);
    virtio::write32(io_base, virtio::REG_GUEST_FEATURES, 0);

    // Read capacity
    let cap_lo = virtio::read32(io_base, CFG_CAPACITY) as u64;
    let cap_hi = virtio::read32(io_base, CFG_CAPACITY + 4) as u64;
    let capacity = cap_lo | (cap_hi << 32);

    // Set up virtqueue 0 (the request queue)
    virtio::write16(io_base, virtio::REG_QUEUE_SELECT, 0);
    let queue_size = virtio::read16(io_base, virtio::REG_QUEUE_SIZE);
    if queue_size == 0 {
        return Err("VirtIO block: queue size is 0");
    }

    // Allocate virtqueue memory (descriptors + avail + used, all page-aligned)
    let pmm = Pmm::get();
    let desc_frame = pmm.alloc().ok_or("VirtIO: out of memory for desc")?;
    let avail_frame = pmm.alloc().ok_or("VirtIO: out of memory for avail")?;
    let used_frame = pmm.alloc().ok_or("VirtIO: out of memory for used")?;

    let desc_phys = desc_frame.start_address().as_u64();
    let avail_phys = avail_frame.start_address().as_u64();
    let used_phys = used_frame.start_address().as_u64();

    let desc = (desc_phys + hhdm_offset) as *mut VringDesc;
    let avail = (avail_phys + hhdm_offset) as *mut VringAvail;
    let used = (used_phys + hhdm_offset) as *mut VringUsed;

    // Tell device where the queue is (legacy: physical page number)
    let queue_pfn = desc_phys / PAGE_SIZE;
    virtio::write32(io_base, virtio::REG_QUEUE_ADDRESS, queue_pfn as u32);

    // Mark driver OK
    virtio::write8(
        io_base,
        virtio::REG_DEVICE_STATUS,
        virtio::STATUS_ACKNOWLEDGE | virtio::STATUS_DRIVER | virtio::STATUS_DRIVER_OK,
    );

    let dev = VirtioBlock {
        io_base,
        capacity_sectors: capacity,
        desc_phys,
        desc,
        avail_phys,
        avail,
        used_phys,
        used,
        queue_size,
        next_desc: 0,
        last_used_idx: 0,
        hhdm_offset,
    };

    crate::serial_println!(
        "VirtIO block: {} sectors ({} MiB), queue_size={}",
        capacity,
        capacity * SECTOR_SIZE / (1024 * 1024),
        queue_size
    );

    *BLOCK_DEV.lock() = Some(dev);
    Ok(())
}

/// Read sectors from the block device.
pub fn read_sectors(start_sector: u64, buf: &mut [u8]) -> Result<(), &'static str> {
    if !buf.len().is_multiple_of(SECTOR_SIZE as usize) {
        return Err("Buffer size must be a multiple of sector size");
    }

    let mut guard = BLOCK_DEV.lock();
    let dev = guard.as_mut().ok_or("VirtIO block not initialized")?;

    let sector_count = buf.len() as u64 / SECTOR_SIZE;
    if start_sector + sector_count > dev.capacity_sectors {
        return Err("Read past end of device");
    }

    do_request(dev, VIRTIO_BLK_T_IN, start_sector, buf)
}

/// Write sectors to the block device.
pub fn write_sectors(start_sector: u64, buf: &[u8]) -> Result<(), &'static str> {
    if !buf.len().is_multiple_of(SECTOR_SIZE as usize) {
        return Err("Buffer size must be a multiple of sector size");
    }

    let mut guard = BLOCK_DEV.lock();
    let dev = guard.as_mut().ok_or("VirtIO block not initialized")?;

    let sector_count = buf.len() as u64 / SECTOR_SIZE;
    if start_sector + sector_count > dev.capacity_sectors {
        return Err("Write past end of device");
    }

    // SAFETY: We need a mutable reference for do_request but the data is read-only.
    // Cast is safe because VIRTIO_BLK_T_OUT only reads from the buffer.
    let buf_mut = unsafe { core::slice::from_raw_parts_mut(buf.as_ptr() as *mut u8, buf.len()) };
    do_request(dev, VIRTIO_BLK_T_OUT, start_sector, buf_mut)
}

/// Get the device capacity in sectors.
pub fn capacity_sectors() -> Option<u64> {
    BLOCK_DEV.lock().as_ref().map(|d| d.capacity_sectors)
}

/// Perform a single block I/O request via the virtqueue.
fn do_request(
    dev: &mut VirtioBlock,
    req_type: u32,
    sector: u64,
    buf: &mut [u8],
) -> Result<(), &'static str> {
    // Allocate a request header on a PMM frame
    let pmm = Pmm::get();
    let header_frame = pmm.alloc().ok_or("VirtIO: out of memory for request")?;
    let header_phys = header_frame.start_address().as_u64();
    let header_virt = (header_phys + dev.hhdm_offset) as *mut VirtioBlkReqHeader;

    // SAFETY: Frame is freshly allocated and mapped via HHDM.
    unsafe {
        (*header_virt).req_type = req_type;
        (*header_virt)._reserved = 0;
        (*header_virt).sector = sector;
    }

    // Status byte at the end of the header page
    let status_phys = header_phys + core::mem::size_of::<VirtioBlkReqHeader>() as u64;
    let status_virt = (status_phys + dev.hhdm_offset) as *mut u8;
    // SAFETY: Within the same allocated frame.
    unsafe {
        *status_virt = 0xFF;
    } // Sentinel

    // Use a temporary PMM frame for DMA-accessible data buffer.
    let data_frame = pmm.alloc().ok_or("VirtIO: out of memory for data")?;
    let data_phys = data_frame.start_address().as_u64();
    let data_virt = (data_phys + dev.hhdm_offset) as *mut u8;

    if req_type == VIRTIO_BLK_T_OUT {
        // Copy data to the DMA-accessible buffer
        // SAFETY: Both buffers are valid and non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), data_virt, buf.len());
        }
    }

    // Set up 3 descriptors: header, data, status
    let d0 = dev.next_desc;
    let d1 = d0 + 1;
    let d2 = d1 + 1;
    dev.next_desc = (d2 + 1) % dev.queue_size;

    // SAFETY: Descriptor indices are within queue bounds.
    unsafe {
        // Descriptor 0: request header (device reads)
        (*dev.desc.add(d0 as usize)) = VringDesc {
            addr: header_phys,
            len: core::mem::size_of::<VirtioBlkReqHeader>() as u32,
            flags: VRING_DESC_F_NEXT,
            next: d1,
        };

        // Descriptor 1: data buffer
        let data_flags = if req_type == VIRTIO_BLK_T_IN {
            VRING_DESC_F_NEXT | VRING_DESC_F_WRITE // Device writes to buffer
        } else {
            VRING_DESC_F_NEXT // Device reads from buffer
        };
        (*dev.desc.add(d1 as usize)) = VringDesc {
            addr: data_phys,
            len: buf.len() as u32,
            flags: data_flags,
            next: d2,
        };

        // Descriptor 2: status (device writes 1 byte)
        (*dev.desc.add(d2 as usize)) = VringDesc {
            addr: status_phys,
            len: 1,
            flags: VRING_DESC_F_WRITE,
            next: 0,
        };

        // Add to available ring
        let avail_idx = (*dev.avail).idx;
        (*dev.avail).ring[(avail_idx % dev.queue_size) as usize] = d0;
        fence(Ordering::SeqCst);
        (*dev.avail).idx = avail_idx.wrapping_add(1);
        fence(Ordering::SeqCst);
    }

    // Notify the device
    virtio::write16(dev.io_base, virtio::REG_QUEUE_NOTIFY, 0);

    // Poll for completion
    let mut spins = 0u64;
    loop {
        fence(Ordering::SeqCst);
        // SAFETY: Reading the used ring index.
        let used_idx = unsafe { (*dev.used).idx };
        if used_idx != dev.last_used_idx {
            dev.last_used_idx = used_idx;
            break;
        }
        spins += 1;
        if spins > 100_000_000 {
            // SAFETY: Free allocated frames on timeout.
            unsafe {
                pmm.dealloc(header_frame);
                pmm.dealloc(data_frame);
            }
            return Err("VirtIO block: request timed out");
        }
        core::hint::spin_loop();
    }

    // Check status
    // SAFETY: Device has written the status byte.
    let status = unsafe { *status_virt };
    if status != 0 {
        // SAFETY: Free allocated frames on error.
        unsafe {
            pmm.dealloc(header_frame);
            pmm.dealloc(data_frame);
        }
        return Err("VirtIO block: request failed");
    }

    if req_type == VIRTIO_BLK_T_IN {
        // Copy data from DMA buffer to caller's buffer
        // SAFETY: Both buffers are valid and non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(data_virt, buf.as_mut_ptr(), buf.len());
        }
    }

    // Free temporary frames
    // SAFETY: Frames were allocated above and are no longer in use.
    unsafe {
        pmm.dealloc(header_frame);
        pmm.dealloc(data_frame);
    }

    Ok(())
}
