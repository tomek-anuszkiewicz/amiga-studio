#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! MOS 8520 CIA Whole-Machine Loop Integration Tests
//!
//! Verifies CIA-A and CIA-B timer underflows, Paula INTREQ cross-chip signaling,
//! CPU IPL 2 and IPL 6 escalation, and 50 Hz VBlank TOD ticking in the machine loop.

mod common;
use common::MachineHarness;

#[test]
fn test_cia_a_timer_underflow_triggers_level2_interrupt() {
    let mut harness = MachineHarness::new();

    // 1. Unmask Level 2 PORTS interrupt in Paula INTENA (bit 3 + master bit 14 = 0x4008)
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x09A, 0xC008); // SET/CLR + INTEN + PORTS

    // 2. Configure CIA-A Timer A:
    // Latch = 5 E-Clocks (each E-Clock = 5 CCKs -> 25 CCKs total)
    harness.machine.cia_a.commit_register_write(0x04, 5); // TALO
    harness.machine.cia_a.commit_register_write(0x05, 0); // TAHI (also reloads ta_counter when stopped)

    // Unmask Timer A underflow in CIA-A ICR (bit 7 SET + bit 0 = 0x81)
    harness.machine.cia_a.commit_register_write(0x0D, 0x81);

    // Assert initially no interrupt is pending
    assert!(!harness.machine.cia_a.irq_pending());
    assert_eq!(harness.machine.paula.pending_interrupt_level(), 0);

    // Start Timer A in continuous mode (CRA bit 0 = 1)
    harness.machine.cia_a.commit_register_write(0x0E, 0x01);

    // Step 4 E-Clocks (20 CCKs): should not have underflowed yet
    harness.step_cck(20);
    assert!(!harness.machine.cia_a.irq_pending());

    // Step across underflow boundary (additional 15 CCKs)
    harness.step_cck(15);

    // Verify CIA-A ICR captured underflow and asserted IRQ
    assert!(
        harness.machine.cia_a.irq_pending(),
        "CIA-A IRQ should be pending after timer underflow"
    );

    // Verify Paula INTREQ received PORTS interrupt (bit 3 = 0x0008)
    assert_ne!(
        harness.machine.paula.interrupts.intreq & 0x0008,
        0,
        "Paula INTREQ bit 3 (PORTS) should be asserted"
    );

    // Verify CPU IPL line is escalated to Level 2
    assert_eq!(
        harness.machine.paula.pending_interrupt_level(),
        2,
        "Machine loop should resolve IPL 2 for CIA-A interrupt"
    );
}

#[test]
fn test_cia_b_timer_underflow_triggers_level6_interrupt() {
    let mut harness = MachineHarness::new();

    // 1. Unmask Level 6 EXTER interrupt in Paula INTENA (bit 13 + master bit 14 = 0x6000)
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x09A, 0xE000); // SET/CLR + INTEN + EXTER

    // 2. Configure CIA-B Timer A for 4 E-Clocks
    harness.machine.cia_b.commit_register_write(0x04, 4); // TALO
    harness.machine.cia_b.commit_register_write(0x05, 0); // TAHI

    // Unmask Timer A in CIA-B ICR (0x81)
    harness.machine.cia_b.commit_register_write(0x0D, 0x81);

    assert!(!harness.machine.cia_b.irq_pending());
    assert_eq!(harness.machine.paula.pending_interrupt_level(), 0);

    // Start Timer A in continuous mode (CRA bit 0 = 1)
    harness.machine.cia_b.commit_register_write(0x0E, 0x01);

    // Step 30 CCKs (6 E-Clocks, guaranteeing underflow)
    harness.step_cck(30);

    // Verify CIA-B IRQ and Paula EXTER (bit 13)
    assert!(harness.machine.cia_b.irq_pending());
    assert_ne!(
        harness.machine.paula.interrupts.intreq & 0x2000,
        0,
        "Paula INTREQ bit 13 (EXTER) should be asserted"
    );

    // Verify CPU IPL line is escalated to Level 6
    assert_eq!(
        harness.machine.paula.pending_interrupt_level(),
        6,
        "Machine loop should resolve IPL 6 for CIA-B interrupt"
    );
}

#[test]
fn test_cia_tod_50hz_vblank_tick() {
    let mut harness = MachineHarness::new();

    let initial_tod = harness.machine.cia_a.tod;

    // Advance machine by 1 full vertical frame (312 scanlines for PAL)
    harness.machine.step_frame();

    let after_tod = harness.machine.cia_a.tod;

    // Assert that the VBlank strobe advanced CIA-A TOD tick counter exactly once
    assert_eq!(
        after_tod,
        initial_tod.wrapping_add(1),
        "CIA-A TOD counter should increment by 1 on vertical frame boundary"
    );
}

#[test]
fn test_poll_peripheral_pins_isolation_from_cia_interrupts() {
    let mut harness = MachineHarness::new();

    // 1. Enable Level 2 PORTS interrupt in Paula INTENA (bit 3 + master bit 14 = 0xC008)
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x09A, 0xC008);

    // 2. Configure CIA-A Timer A and cause underflow
    harness.machine.cia_a.commit_register_write(0x04, 1);
    harness.machine.cia_a.commit_register_write(0x05, 0);
    harness.machine.cia_a.commit_register_write(0x0D, 0x81);
    harness.machine.cia_a.commit_register_write(0x0E, 0x01);

    harness.step_cck(25);
    assert!(harness.machine.cia_a.irq_pending());

    // 3. Clear Paula INTREQ bit 3 to simulate clean state
    harness.machine.paula.commit_register_write(0x09C, 0x0008);
    assert_eq!(harness.machine.paula.interrupts.intreq & 0x0008, 0);

    // 4. Calling poll_peripheral_pins() must NOT assert Paula's INT2 pin
    harness.machine.poll_peripheral_pins();
    assert_eq!(
        harness.machine.paula.interrupts.intreq & 0x0008,
        0,
        "poll_peripheral_pins must not route CIA IRQ to Paula"
    );

    // 5. Stepping the CCK machine loop routes CIA IRQ to Paula's INT2 pin
    harness.step_cck(2);
    assert_ne!(
        harness.machine.paula.interrupts.intreq & 0x0008,
        0,
        "Machine loop CCK stepping must route CIA IRQ to Paula INT2"
    );
}
