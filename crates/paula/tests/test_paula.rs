use paula::Paula;

#[test]
fn test_interrupt_arbitration_and_set_clr() {
    let mut paula = Paula::new();
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Enable master INTEN (bit 14) + VBlank (bit 5) + Audio 0 (bit 7)
    paula.write_intena(0xC0A0); // $8000 | 0x4000 | 0x0080 | 0x0020
    assert_eq!(paula.intena, 0x40A0);

    // Request VBlank (Level 3)
    paula.set_interrupt_request(0x0020);
    assert_eq!(paula.pending_interrupt_level(), 3);

    // Request Audio 0 (Level 4, higher priority)
    paula.set_interrupt_request(0x0080);
    assert_eq!(paula.pending_interrupt_level(), 4);

    // Clear Audio 0 request
    paula.clear_interrupt_request(0x0080);
    assert_eq!(paula.pending_interrupt_level(), 3);

    // Clear master enable
    paula.write_intena(0x4000); // Clear bit 14
    assert_eq!(paula.pending_interrupt_level(), 0);
}
