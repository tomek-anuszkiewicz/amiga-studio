#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cia::{Cia, CiaId};

#[test]
fn test_cia_timer_and_icr() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Enable Timer A interrupt in ICR: $80 | 0x01 = $81
    cia.write_register(0xD, 0x81);
    assert_eq!(cia.icr_mask, 0x01);

    // Configure Timer A latch = 2, start timer in continuous mode (CRA = $01)
    cia.write_register(0x4, 2);
    cia.write_register(0x5, 0);
    cia.write_register(0xE, 0x01);

    assert!(!cia.irq_pending());

    // Step 15 CCKs (3 E-Clocks: 2 -> 1 -> 0 -> underflow)
    for _ in 0..15 {
        cia.step_cck();
    }

    assert!(cia.irq_pending());
    // Read ICR clears request
    assert_eq!(cia.read_register(0xD) & 0x81, 0x81);
    assert!(!cia.irq_pending());
}

#[test]
fn test_cia_read_register_debug() {
    let mut cia = Cia::new(CiaId::A);
    cia.pra = 0x42;
    assert_eq!(cia.read_register_debug(0), 0x42);
    assert_eq!(cia.read_register_debug(0), cia.peek_register(0));
}

#[test]
fn test_cia_b_initialization() {
    let cia_b = Cia::new(CiaId::B);
    assert_eq!(cia_b.id, CiaId::B);
    assert_eq!(cia_b.icr_mask, 0);
}

#[test]
fn test_cia_id_default() {
    assert_eq!(CiaId::default(), CiaId::A);
}
