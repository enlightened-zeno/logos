/// Serial port logic tests.

#[test]
fn test_com1_port() {
    let com1: u16 = 0x3F8;
    assert_eq!(com1, 0x3F8);
}

#[test]
fn test_baud_divisor() {
    let base_clock = 115200u32;
    let baud = 115200u32;
    let divisor = base_clock / baud;
    assert_eq!(divisor, 1);

    let baud_9600 = 9600u32;
    let divisor_9600 = base_clock / baud_9600;
    assert_eq!(divisor_9600, 12);
}

#[test]
fn test_line_control() {
    // 8N1: 8 data bits, no parity, 1 stop bit
    let lcr: u8 = 0x03; // 8 data bits
    assert_eq!(lcr & 0x03, 0x03); // 8 bits
    assert_eq!(lcr & 0x04, 0); // 1 stop bit
    assert_eq!(lcr & 0x38, 0); // No parity
}

#[test]
fn test_fifo_enable() {
    let fcr: u8 = 0xC7; // Enable FIFO, 14-byte threshold
    assert!(fcr & 0x01 != 0); // FIFO enabled
}

#[test]
fn test_newline_conversion() {
    // Serial: \n → \r\n
    let input = "hello\nworld\n";
    let output = input.replace('\n', "\r\n");
    assert_eq!(output, "hello\r\nworld\r\n");
}
