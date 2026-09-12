use parallel_port::ParallelPort;

#[test]
fn test_parallel_port_data_and_reset() {
    let mut port = ParallelPort::new();
    assert!(port.select);

    port.write_data(0x55);
    assert_eq!(port.read_data(), 0x55);

    port.reset();
    assert_eq!(port.read_data(), 0);
    assert!(port.select);
}
