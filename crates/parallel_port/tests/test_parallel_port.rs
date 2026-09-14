use parallel_port::ParallelPort;

#[test]
fn test_parallel_port_data_read_write() {
    let mut port = ParallelPort::new();
    assert_eq!(port.read_data(), 0x00);

    port.write_data(0xAA);
    assert_eq!(port.read_data(), 0xAA);

    port.write_data(0x55);
    assert_eq!(port.read_data(), 0x55);

    port.write_data(0xFF);
    assert_eq!(port.read_data(), 0xFF);
}

#[test]
fn test_parallel_port_direction_and_data_masking() {
    let mut port = ParallelPort::new();
    assert_eq!(port.direction, 0x00); // Default input mode

    // Configure all lines as output
    port.direction = 0xFF;
    port.write_data(0xC3);
    assert_eq!(port.read_data(), 0xC3);

    // Mixed direction (e.g. nibbles: high output, low input)
    port.direction = 0xF0;
    assert_eq!(port.direction, 0xF0);
}

#[test]
fn test_parallel_port_handshake_signals() {
    let mut port = ParallelPort::new();

    // Standard initial states
    assert!(port.select); // Online by default
    assert!(!port.strobe);
    assert!(!port.busy);
    assert!(!port.paper_out);

    // Emulate printer transaction sequence
    // Host asserts strobe (active low line represented by true boolean)
    port.strobe = true;
    assert!(port.strobe);

    // Peripheral asserts BUSY
    port.busy = true;
    assert!(port.busy);

    // Host negates strobe
    port.strobe = false;
    assert!(!port.strobe);

    // Peripheral clears BUSY
    port.busy = false;
    assert!(!port.busy);

    // Paper out indication
    port.paper_out = true;
    assert!(port.paper_out);

    // Offline indication
    port.select = false;
    assert!(!port.select);
}

#[test]
fn test_parallel_port_reset_and_defaults() {
    let mut port = ParallelPort::new();
    port.write_data(0x7F);
    port.direction = 0x55;
    port.strobe = true;
    port.busy = true;
    port.paper_out = true;
    port.select = false;

    port.reset();
    assert_eq!(port.read_data(), 0x00);
    assert_eq!(port.data, 0x00);
    assert_eq!(port.direction, 0x00);
    assert!(!port.strobe);
    assert!(!port.busy);
    assert!(!port.paper_out);
    assert!(port.select); // Re-asserts select on reset

    assert_eq!(port, ParallelPort::new());
}
