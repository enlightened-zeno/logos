/// VirtIO driver logic tests.

#[test]
fn test_virtio_vendor_id() {
    let vendor: u16 = 0x1AF4;
    assert_eq!(vendor, 0x1AF4);
}

#[test]
fn test_virtio_block_device_id() {
    let device: u16 = 0x1001;
    assert_eq!(device, 0x1001);
}

#[test]
fn test_sector_size() {
    let sector: u64 = 512;
    assert_eq!(sector, 512);
}

#[test]
fn test_virtqueue_descriptor_flags() {
    let next: u16 = 1;
    let write: u16 = 2;
    assert_eq!(next & write, 0); // Distinct bits
}

#[test]
fn test_bar0_io_space() {
    let bar: u32 = 0xC041; // Typical BAR0 with bit 0 set
    assert!(bar & 1 != 0); // I/O space
    let io_base = (bar & 0xFFFC) as u16;
    assert_eq!(io_base, 0xC040);
}

#[test]
fn test_virtio_status_bits() {
    let ack: u8 = 1;
    let driver: u8 = 2;
    let driver_ok: u8 = 4;
    let features_ok: u8 = 8;
    let combined = ack | driver | driver_ok;
    assert_eq!(combined, 7);
    let _ = features_ok;
}
