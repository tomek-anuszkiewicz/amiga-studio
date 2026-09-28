#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use paula::Paula;

#[test]
fn test_interrupt_arbitration_and_set_clr() {
    let mut paula = Paula::new();
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Enable master INTEN (bit 14) + VBlank (bit 5) + Audio 0 (bit 7)
    paula.write_intena(0xC0A0); // $8000 | 0x4000 | 0x0080 | 0x0020
    assert_eq!(paula.interrupts.intena, 0x40A0);

    // Request VBlank (Level 3)
    paula.set_interrupt_request(0x0020);
    assert_eq!(paula.pending_interrupt_level(), 3);

    // Request Audio 0 (Level 4, higher priority)
    paula.set_interrupt_request(0x0080);
    assert_eq!(paula.pending_interrupt_level(), 4);

    // Clear Audio 0 request
    paula.write_intreq(0x0080);
    assert_eq!(paula.pending_interrupt_level(), 3);

    // Clear master enable
    paula.write_intena(0x4000); // Clear bit 14
    assert_eq!(paula.pending_interrupt_level(), 0);
}

#[test]
fn test_paula_reset_state() {
    let mut paula = Paula::new();
    paula.interrupts.intena = 0xFFFF;
    paula.reset();
    assert_eq!(paula.interrupts.intena, 0);
    assert_eq!(paula.pending_interrupt_level(), 0);
    assert_eq!(Paula::MUTATION_CAPACITY, 32);
}

#[test]
fn test_paula_int2_int6_pins_delay_and_arbitration() {
    let mut paula = Paula::new();

    // Enable master INTEN + PORTS (Level 2) + EXTER (Level 6)
    // 0x8000 (SET) | 0x4000 (INTEN) | 0x2000 (EXTER) | 0x0008 (PORTS) = 0xE008
    paula.write_intena(0xE008);
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Assert _INT2 pin (CIA-A) -> stages mutation for 1 CCK
    paula.set_int2_pin(true);
    // Before CCK step, mutation has not matured yet
    assert_eq!(paula.interrupts.intreq & 0x0008, 0);
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Step 1 CCK -> mutation matures into INTREQ bit 3
    paula.step_cck();
    assert_ne!(paula.interrupts.intreq & 0x0008, 0);
    assert_eq!(paula.pending_interrupt_level(), 2);

    // Assert _INT6 pin (CIA-B) -> stages mutation for 1 CCK
    paula.set_int6_pin(true);
    // Before step, level remains 2
    assert_eq!(paula.pending_interrupt_level(), 2);

    // Step 1 CCK -> EXTER matures into INTREQ bit 13
    paula.step_cck();
    assert_ne!(paula.interrupts.intreq & 0x2000, 0);
    // Level 6 takes priority over Level 2
    assert_eq!(paula.pending_interrupt_level(), 6);

    // Clear Level 6 in INTREQ (bit 15 = 0, bit 13 = EXTER)
    paula.write_intreq(0x2000);
    // Drops back to Level 2
    assert_eq!(paula.pending_interrupt_level(), 2);
}
