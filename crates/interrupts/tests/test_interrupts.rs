#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use interrupts::*;

#[test]
fn test_all_fourteen_interrupt_bit_definitions() {
    assert_eq!(IRQ_TBE, 1 << 0);
    assert_eq!(IRQ_DSKBLK, 1 << 1);
    assert_eq!(IRQ_SOFT, 1 << 2);
    assert_eq!(IRQ_PORTS, 1 << 3);
    assert_eq!(IRQ_COPER, 1 << 4);
    assert_eq!(IRQ_VERTB, 1 << 5);
    assert_eq!(IRQ_BLIT, 1 << 6);
    assert_eq!(IRQ_AUD0, 1 << 7);
    assert_eq!(IRQ_AUD1, 1 << 8);
    assert_eq!(IRQ_AUD2, 1 << 9);
    assert_eq!(IRQ_AUD3, 1 << 10);
    assert_eq!(IRQ_RBF, 1 << 11);
    assert_eq!(IRQ_DSKSYN, 1 << 12);
    assert_eq!(IRQ_EXTER, 1 << 13);
}

#[test]
fn test_intena_set_clr_semantics() {
    let mut ctrl = InterruptController::new();
    assert_eq!(ctrl.intena, 0);
    assert_eq!(ctrl.read_intenar(), 0);
    assert!(!ctrl.is_master_enabled());

    // Write with SET (bit 15 = 1): enable Master (bit 14) and VERTB (bit 5)
    ctrl.write_intena(INT_SET_CLR | INTENA_INTEN | IRQ_VERTB);
    assert_eq!(ctrl.intena, 0x4020);
    assert!(ctrl.is_master_enabled());
    assert_ne!(ctrl.intena & IRQ_VERTB, 0);
    assert_eq!(ctrl.intena & IRQ_COPER, 0);

    // Write with SET: enable COPER (bit 4) without altering VERTB or INTEN
    ctrl.write_intena(INT_SET_CLR | IRQ_COPER);
    assert_eq!(ctrl.intena, 0x4030);
    assert_ne!(ctrl.intena & IRQ_COPER, 0);
    assert_ne!(ctrl.intena & IRQ_VERTB, 0);

    // Write with CLR (bit 15 = 0): clear VERTB without altering COPER or INTEN
    ctrl.write_intena(IRQ_VERTB);
    assert_eq!(ctrl.intena, 0x4010);
    assert_eq!(ctrl.intena & IRQ_VERTB, 0);
    assert_ne!(ctrl.intena & IRQ_COPER, 0);

    // Clear master enable bit 14
    ctrl.write_intena(INTENA_INTEN);
    assert_eq!(ctrl.intena, 0x0010);
    assert!(!ctrl.is_master_enabled());
}

#[test]
fn test_intreq_set_clr_and_request_methods() {
    let mut ctrl = InterruptController::new();
    assert_eq!(ctrl.intreq, 0);
    assert_eq!(ctrl.read_intreqr(), 0);

    // Assert request via SET write
    ctrl.write_intreq(INT_SET_CLR | IRQ_BLIT | IRQ_PORTS);
    assert_eq!(ctrl.intreq, IRQ_BLIT | IRQ_PORTS);
    assert_ne!(ctrl.intreq & IRQ_BLIT, 0);
    assert_ne!(ctrl.intreq & IRQ_PORTS, 0);
    assert_eq!(ctrl.intreq & IRQ_VERTB, 0);

    // Clear BLIT via CLR write
    ctrl.write_intreq(IRQ_BLIT);
    assert_eq!(ctrl.intreq, IRQ_PORTS);
    assert_eq!(ctrl.intreq & IRQ_BLIT, 0);
    assert_ne!(ctrl.intreq & IRQ_PORTS, 0);

    // Direct request() call (custom chip line assertion)
    ctrl.request(IRQ_AUD0 | IRQ_AUD1);
    assert_eq!(ctrl.intreq, IRQ_PORTS | IRQ_AUD0 | IRQ_AUD1);
    assert_ne!(ctrl.intreq & IRQ_AUD0, 0);

    // Clear AUD0 via CLR write
    ctrl.write_intreq(IRQ_AUD0);
    assert_eq!(ctrl.intreq, IRQ_PORTS | IRQ_AUD1);
    assert_eq!(ctrl.intreq & IRQ_AUD0, 0);
    assert_ne!(ctrl.intreq & IRQ_AUD1, 0);
}

#[test]
fn test_priority_encoder_levels_and_preemption() {
    let mut ctrl = InterruptController::new();

    // With master INTEN cleared, pending_level() must return 0 even if requests exist
    ctrl.request(IRQ_EXTER | IRQ_BLIT | IRQ_TBE);
    ctrl.intena = IRQ_EXTER | IRQ_BLIT | IRQ_TBE; // Note: INTEN (bit 14) is 0
    assert_eq!(ctrl.pending_level(), 0);
    assert_eq!(ctrl.pending_mask(), 0);

    // Enable master INTEN
    ctrl.intena |= INTENA_INTEN;
    assert_eq!(ctrl.pending_level(), 6); // Level 6 (EXTER) preempts Level 3 and 1

    // Clear Level 6
    ctrl.write_intreq(IRQ_EXTER);
    assert_eq!(ctrl.pending_level(), 3); // Level 3 (BLIT) preempts Level 1

    // Request Level 5 (DSKSYN)
    ctrl.intena |= IRQ_DSKSYN;
    ctrl.request(IRQ_DSKSYN);
    assert_eq!(ctrl.pending_level(), 5); // Level 5 preempts Level 3

    // Clear Level 5
    ctrl.write_intreq(IRQ_DSKSYN);
    assert_eq!(ctrl.pending_level(), 3);

    // Request Level 4 (AUD2)
    ctrl.intena |= IRQ_AUD2;
    ctrl.request(IRQ_AUD2);
    assert_eq!(ctrl.pending_level(), 4);

    // Clear Level 4 and Level 3
    ctrl.write_intreq(IRQ_AUD2);
    ctrl.write_intreq(IRQ_BLIT);

    // Request Level 2 (PORTS)
    ctrl.intena |= IRQ_PORTS;
    ctrl.request(IRQ_PORTS);
    assert_eq!(ctrl.pending_level(), 2);

    // Clear Level 2, leave only Level 1 (TBE)
    ctrl.write_intreq(IRQ_PORTS);
    assert_eq!(ctrl.pending_level(), 1);

    // Clear Level 1
    ctrl.write_intreq(IRQ_TBE);
    assert_eq!(ctrl.pending_level(), 0);
}

#[test]
fn test_controller_reset() {
    let mut ctrl = InterruptController::new();
    ctrl.write_intena(INT_SET_CLR | 0x7FFF);
    ctrl.write_intreq(INT_SET_CLR | 0x7FFF);
    assert_ne!(ctrl.intena, 0);
    assert_ne!(ctrl.intreq, 0);

    ctrl.reset();
    assert_eq!(ctrl.intena, 0);
    assert_eq!(ctrl.intreq, 0);
    assert_eq!(ctrl.pending_level(), 0);
    assert_eq!(ctrl.pending_mask(), 0);
}
