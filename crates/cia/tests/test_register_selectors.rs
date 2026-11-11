use cia::Cia;

#[test]
fn named_setters_capture_only_on_the_falling_edge() {
    type Case = (&'static str, fn(&mut Cia, u8) -> bool, fn(&Cia) -> u32, u32);
    let cases: [Case; 15] = [
        ("PRA", Cia::set_pra, |c| u32::from(c.pra), 0x5a),
        ("PRB", Cia::set_prb, |c| u32::from(c.prb), 0x5a),
        ("DDRA", Cia::set_ddra, |c| u32::from(c.ddra), 0x5a),
        ("DDRB", Cia::set_ddrb, |c| u32::from(c.ddrb), 0x5a),
        ("TALO", Cia::set_talo, |c| u32::from(c.ta_latch), 0xff5a),
        ("TAHI", Cia::set_tahi, |c| u32::from(c.ta_latch), 0x5aff),
        ("TBLO", Cia::set_tblo, |c| u32::from(c.tb_latch), 0xff5a),
        ("TBHI", Cia::set_tbhi, |c| u32::from(c.tb_latch), 0x5aff),
        ("TODLO", Cia::set_todlo, |c| c.tod, 0x5a),
        ("TODMID", Cia::set_todmid, |c| c.tod, 0x5a00),
        ("TODHI", Cia::set_todhi, |c| c.tod, 0x5a0000),
        ("SDR", Cia::set_sdr, |c| u32::from(c.sdr), 0x5a),
        ("ICR", Cia::set_icr, |c| u32::from(c.icr_mask), 5),
        ("CRA", Cia::set_cra, |c| u32::from(c.cra), 0x4a),
        ("CRB", Cia::set_crb, |c| u32::from(c.crb), 0x4a),
    ];
    for (name, setter, observe, expected) in cases {
        let mut cia = Cia::new();
        if name == "ICR" {
            cia.icr_mask = 0x1f;
        }
        let before = observe(&cia);
        assert!(setter(&mut cia, 0x5a), "{name}");
        assert_eq!(cia.mutations.iter().flatten().count(), 1, "{name}");
        assert_eq!(observe(&cia), before, "{name}: scheduling");
        cia.step_cck();
        assert_eq!(observe(&cia), before, "{name}: before falling edge");
        cia.step_cck();
        assert_eq!(observe(&cia), expected, "{name}: falling edge");
        assert!(cia.mutations.iter().all(Option::is_none), "{name}");
    }
}

#[test]
fn full_queue_rejects_without_early_application_and_drains_every_write() {
    let mut cia = Cia::new();
    for value in 1..=16 {
        assert!(cia.set_pra(value));
    }
    assert!(!cia.set_pra(0xee));
    assert_eq!(cia.pra(), 0);
    cia.step_cck();
    assert_eq!(cia.pra(), 0);
    cia.step_cck();
    assert_eq!(cia.pra(), 16);
    assert_eq!(cia.prev_pra, 15);
    assert!(cia.mutations.iter().all(Option::is_none));
    assert_eq!(cia.poll_pra_output(), Some(16));
    assert_eq!(cia.poll_pra_output(), None);
    assert!(cia.set_icr(0x81));
    assert!(cia.set_icr(0x01));
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(cia.icr_mask, 0);
}

#[test]
fn debug_reads_leave_pending_work_and_read_side_effects_untouched() {
    let mut cia = Cia::new();
    cia.tod = 0x123456;
    cia.icr_data = 0x81;
    assert!(cia.set_ddra(1));
    let before = cia.clone();
    assert_eq!(cia.todhi_debug(), 0x12);
    assert_eq!(cia.todmid_debug(), 0x34);
    assert_eq!(cia.todlo_debug(), 0x56);
    assert_eq!(cia.icr_debug(), 0x81);
    assert_eq!(cia, before);
    assert_eq!(cia.todhi(), 0x12);
    assert!(!cia.tod_latched);
    cia.step_cck();
    assert!(!cia.tod_latched);
    cia.step_cck();
    assert!(cia.tod_latched);
    cia.tod = 0xabcdef;
    assert_eq!(cia.todmid(), 0x34);
    assert_eq!(cia.todlo(), 0x56);
    assert!(cia.tod_latched);
    for _ in 0..5 {
        cia.step_cck();
    }
    assert!(!cia.tod_latched);
    assert_eq!(cia.icr(), 0x81);
    assert_eq!(cia.icr_debug(), 0);
}
