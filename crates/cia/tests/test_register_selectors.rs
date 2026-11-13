#![allow(clippy::unwrap_used)]

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
        assert_eq!(cia.mutations.len(), 1, "{name}");
        assert_eq!(observe(&cia), before, "{name}: scheduling");
        cia.step_cck();
        assert_eq!(observe(&cia), before, "{name}: before falling edge");
        cia.step_cck();
        assert_eq!(observe(&cia), expected, "{name}: falling edge");
        assert!(cia.mutations.is_empty(), "{name}");
    }
}

#[test]
fn last_pending_value_is_captured_at_the_same_falling_edge() {
    for phase in 0..5 {
        let mut cia = Cia::new();
        for _ in 0..phase {
            cia.step_cck();
        }
        assert!(cia.set_pra(0x11));
        let delay = cia.mutations.pending().iter().next().unwrap().remaining_cck;
        if delay > 1 {
            cia.step_cck();
        }
        assert!(cia.set_pra(0x22));
        assert!(cia.set_prb(0x33));
        assert_eq!(cia.mutations.len(), 2);
        let remaining = delay - u8::from(delay > 1);
        for _ in 1..remaining {
            cia.step_cck();
            assert_eq!(cia.pra, 0);
        }
        cia.step_cck();
        assert_eq!((cia.pra, cia.prev_pra, cia.prb), (0x22, 0, 0x33));
        assert!(cia.mutations.is_empty());
        assert!(cia.set_pra(0x44));
        assert_eq!(cia.pra, 0x22, "captured state survives a new pending write");
    }
}

#[test]
fn cia_replaces_pending_mask_operations_but_preserves_captured_effects() {
    let mut cia = Cia::new();
    assert!(cia.set_icr(0x81));
    assert!(cia.set_icr(0x82));
    cia.step_cck();
    cia.step_cck();
    assert_eq!(cia.icr_mask, 2, "only the last pre-capture operation acts");
    assert!(cia.set_icr(0x84));
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(
        cia.icr_mask, 6,
        "a captured SET remains composed with later SETs"
    );
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
