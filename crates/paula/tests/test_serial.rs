use paula::serial::SerialPort;

#[test]
fn test_serial_port_reset_and_tbe_status() {
    let mut port = SerialPort::new();
    // Bit 13: TBE (Transmitter Buffer Empty) defaults asserted
    // Bit 12: TSRE (Transmitter Shift Register Empty) defaults asserted
    assert_eq!(port.serdatr & 0x3000, 0x3000);
    assert_eq!(port.serdat, 0);
    assert_eq!(port.serper, 0);

    // Writing to SERDAT clears TBE (bit 13)
    port.write_serdat(0x0141); // 'A' with 9-bit framing stop bit
    assert_eq!(port.serdat, 0x0141);
    assert_eq!(port.serdatr & 0x2000, 0); // TBE cleared
    assert_eq!(port.serdatr & 0x1000, 0x1000); // TSRE remains asserted

    // Reset restores initial TBE and TSRE flags
    port.reset();
    assert_eq!(port.serdatr & 0x3000, 0x3000);
    assert_eq!(port.serdat, 0);
    assert_eq!(port.serper, 0);
}

#[test]
fn test_serial_port_serper_baud_divisor() {
    let mut port = SerialPort::new();
    assert_eq!(port.serper, 0);

    // Standard 9600 baud divisor on PAL (~3.546895 MHz / 9600 - 1 = ~368 = 0x0170)
    port.write_serper(0x0170);
    assert_eq!(port.serper, 0x0170);

    // 9-bit framing bit (bit 15 of SERPER)
    port.write_serper(0x8170);
    assert_eq!(port.serper, 0x8170);
    assert_eq!(port.serper & 0x8000, 0x8000);
}

#[test]
fn test_serial_port_modem_handshake_lines() {
    let mut port = SerialPort::new();
    assert!(!port.cts);
    assert!(!port.rts);
    assert!(!port.dsr);
    assert!(!port.cd);

    // Assert Request To Send
    port.rts = true;
    assert!(port.rts);

    // Host receives Clear To Send from modem
    port.cts = true;
    assert!(port.cts);

    // Data Set Ready and Carrier Detect
    port.dsr = true;
    port.cd = true;
    assert!(port.dsr);
    assert!(port.cd);

    // Reset clears all handshake signals
    port.reset();
    assert!(!port.cts);
    assert!(!port.rts);
    assert!(!port.dsr);
    assert!(!port.cd);
}

#[test]
fn test_serial_port_step_cck_and_equality() {
    let mut port = SerialPort::new();
    port.write_serdat(0x0042);
    port.write_serper(0x00A0);

    // step_cck executes safely without panic
    for _ in 0..100 {
        port.step_cck();
    }

    let cloned = port.clone();
    assert_eq!(port, cloned);
}
