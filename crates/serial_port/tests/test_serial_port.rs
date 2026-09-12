use serial_port::SerialPort;

#[test]
fn test_serial_port_reset_and_tbe() {
    let mut port = SerialPort::new();
    // TBE (bit 13) and TSRE (bit 12) asserted initially
    assert_eq!(port.serdatr & 0x3000, 0x3000);

    port.write_serdat(0x0141); // 'A' with 9-bit framing
    assert_eq!(port.serdatr & 0x2000, 0); // TBE cleared

    port.reset();
    assert_eq!(port.serdatr & 0x3000, 0x3000);
}
