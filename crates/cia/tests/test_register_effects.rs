use cia::Cia;
fn capture(cia: &mut Cia) {
    cia.step_cck();
    while cia.eclock_phase != 2 {
        cia.step_cck();
    }
}

#[test]
fn force_load_is_a_strobe_and_reads_as_zero() {
    let mut cia = Cia::new();
    assert!(cia.set_talo(0x34));
    capture(&mut cia);
    assert!(cia.set_tahi(0x12));
    capture(&mut cia);
    cia.ta_counter = 0;
    assert!(cia.set_cra(0x10));
    capture(&mut cia);
    assert_eq!(cia.ta_counter, 0x1234);
    // HRM Appendix F, Control Register A/B: LOAD has no data storage.
    assert_eq!(cia.cra() & 0x10, 0);
    assert_eq!(cia.cra() & 0x10, 0);

    assert!(cia.set_tblo(0x78));
    capture(&mut cia);
    assert!(cia.set_tbhi(0x56));
    capture(&mut cia);
    cia.tb_counter = 0;
    assert!(cia.set_crb(0x10));
    capture(&mut cia);
    assert_eq!(cia.tb_counter, 0x5678);
    assert_eq!(cia.crb() & 0x10, 0);
    assert_eq!(cia.crb() & 0x10, 0);
}

#[test]
fn writing_tod_middle_stops_time_until_low_byte_write() {
    let mut cia = Cia::new();
    assert!(cia.set_todlo(0x12));
    capture(&mut cia);
    assert!(cia.set_todmid(0x34));
    capture(&mut cia);
    // HRM Appendix F, TOD: any time write stops it until the LSB is written.
    assert!(cia.tod_halted);
    let before_tick = cia.tod;
    cia.tick_tod();
    assert_eq!(cia.tod, before_tick);
    assert!(cia.set_todlo(0x56));
    capture(&mut cia);
    assert!(!cia.tod_halted);
    cia.tick_tod();
    assert_eq!(cia.tod, 0x3457);
}

#[test]
fn enabling_a_pending_source_asserts_the_master_interrupt() {
    let mut cia = Cia::new();
    cia.trigger_icr(0x08);
    assert_eq!(cia.icr_debug(), 0x08);
    assert!(cia.set_icr(0x88));
    assert!(!cia.irq_pending());
    capture(&mut cia);
    assert!(cia.irq_pending());
    assert_eq!(cia.icr(), 0x88);
    assert!(!cia.irq_pending());
}

#[test]
fn timer_high_write_loads_and_starts_one_shot_timers() {
    for timer_b in [false, true] {
        let mut cia = Cia::new();
        if timer_b {
            assert!(cia.set_crb(0x08));
            assert!(cia.set_tblo(2));
        } else {
            assert!(cia.set_cra(0x08));
            assert!(cia.set_talo(2));
        }
        capture(&mut cia);
        // HRM Appendix F, one-shot: high-byte write starts regardless of START.
        if timer_b {
            assert!(cia.set_tbhi(0));
        } else {
            assert!(cia.set_tahi(0));
        }
        capture(&mut cia);
        if timer_b {
            assert_eq!(cia.crb() & 1, 1);
            assert_eq!(cia.tb_counter, 2);
        } else {
            assert_eq!(cia.cra() & 1, 1);
            assert_eq!(cia.ta_counter, 2);
        }
        for _ in 0..3 {
            cia.step_cck();
        }
        assert_eq!(
            if timer_b {
                cia.tb_counter
            } else {
                cia.ta_counter
            },
            1
        );
    }
}

#[test]
fn unused_cra_bit_is_not_stored_or_read_back() {
    let mut cia = Cia::new();
    // HRM Appendix F, CRA: unused bits are unaffected by write and read zero.
    assert!(cia.set_cra(0x80));
    capture(&mut cia);
    assert_eq!(cia.cra(), 0);
    assert_eq!(cia.cra, 0);
}

#[test]
fn port_b_output_notifications_apply_direction_per_pin() {
    let mut cia = Cia::new();
    assert!(cia.set_ddrb(0x0f));
    assert!(cia.set_prb(0x5a));
    assert_eq!(cia.poll_prb_output(), None);
    capture(&mut cia);
    assert_eq!(cia.poll_prb_output(), Some(0xfa));
    assert_eq!(cia.poll_prb_output(), None);
    assert!(cia.set_prb(0));
    capture(&mut cia);
    assert_eq!(cia.poll_prb_output(), Some(0xf0));
    assert!(cia.set_ddrb(0));
    capture(&mut cia);
    assert_eq!(cia.poll_prb_output(), Some(0xff));
}
