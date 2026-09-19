#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cia::{Cia, CiaId};
use config::MutationMode;

#[test]
fn test_tod_atomic_latch_on_read() {
    let mut cia = Cia::new(CiaId::A);
    cia.tod = 0x123456;

    // Reading TODHI ($0A) freezes TOD snapshot
    let hi = cia.read_register(0x0A);
    assert_eq!(hi, 0x12);

    // Counter increments while CPU is fetching lower bytes
    cia.tod = 0x129999;

    // TODMID ($09) returns frozen value ($34), not the incremented value ($99)
    let mid = cia.read_register(0x09);
    assert_eq!(mid, 0x34);

    // TODLO ($08) returns frozen value ($56) and unfreezes
    let lo = cia.read_register(0x08);
    assert_eq!(lo, 0x56);
    assert!(!cia.tod_latched);

    // Subsequent read returns live counter
    assert_eq!(cia.read_register(0x08), 0x99);
}

#[test]
fn test_icr_clear_on_read_vs_peek() {
    let mut cia = Cia::new(CiaId::A);
    cia.icr_mask = 0x01; // Enable Timer A interrupt in mask
    cia.trigger_icr(0x01); // Timer A underflow
    assert!(cia.irq_pending());

    // peek_register should NOT clear ICR
    assert_eq!(cia.peek_register(0x0D) & 0x01, 0x01);
    assert!(cia.irq_pending());

    // read_register clears ICR
    assert_eq!(cia.read_register(0x0D) & 0x01, 0x01);
    assert!(!cia.irq_pending());
}

#[test]
fn test_pin_transitions_ovl_and_led() {
    let mut cia = Cia::new(CiaId::A);
    cia.pra = 0x00;
    cia.prev_pra = 0x00;

    // Toggle _OVL (bit 0): Chip RAM engaged
    cia.write_register(0x0, 0x01);
    assert_eq!(cia.ovl_transition(), Some(true));

    // Toggle back: Kickstart ROM overlay engaged
    cia.write_register(0x0, 0x00);
    assert_eq!(cia.ovl_transition(), Some(false));

    // Toggle _LED (bit 1)
    cia.write_register(0x0, 0x02);
    assert_eq!(cia.led_transition(), Some(true));
}

#[test]
fn test_cia_stage_write_eclock_delay() {
    let mut cia = Cia::new(CiaId::A);
    cia.stage_write(0x2, 0x03, 1, MutationMode::OverwritePending);

    // Initial DDRA is 0
    assert_eq!(cia.ddra, 0);

    // Step 4 CCKs (still in same E-clock subphase)
    for _ in 0..4 {
        cia.step_cck();
    }
    assert_eq!(cia.ddra, 0);

    // 5th CCK completes 1 E-Clock cycle, committing the write!
    cia.step_cck();
    assert_eq!(cia.ddra, 0x03);
}

#[test]
fn test_cia_port_output_polling() {
    let mut cia = Cia::new(CiaId::B);

    // Initial state: no mutations
    assert_eq!(cia.poll_pra_output(), None);
    assert_eq!(cia.poll_prb_output(), None);

    // Write Port B ($BFD100)
    cia.commit_register_write(0x1, 0x75);
    assert_eq!(cia.poll_prb_output(), Some(0x75));
    // Cleared on poll
    assert_eq!(cia.poll_prb_output(), None);

    // Write Port A
    cia.commit_register_write(0x0, 0x5A);
    assert_eq!(cia.poll_pra_output(), Some(0x5A));
    assert_eq!(cia.poll_pra_output(), None);
}
