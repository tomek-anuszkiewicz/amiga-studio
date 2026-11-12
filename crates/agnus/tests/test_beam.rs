use agnus::{Agnus, AgnusModel};

#[test]
fn field_lengths_and_line_alternation_follow_standard_and_lace() {
    for (model, short_lines) in [
        (AgnusModel::OcsPal8371, 312),
        (AgnusModel::OcsNtsc8370, 262),
    ] {
        for interlace in [false, true] {
            let mut a = Agnus::new(model);
            a.dma.bplcon0 = if interlace { 4 } else { 0 };
            let mut elapsed_lines = 0;
            for field in 0..4 {
                let expected_lines = short_lines + if interlace { field % 2 } else { 0 };
                for line in 0..expected_lines {
                    let expected_ccks = 227
                        + if model == AgnusModel::OcsNtsc8370 {
                            elapsed_lines % 2
                        } else {
                            0
                        };
                    for h in 1..expected_ccks {
                        a.step_cck();
                        assert_eq!((a.hpos, a.vpos), (h, line));
                    }
                    a.step_cck();
                    elapsed_lines += 1;
                    assert_eq!(a.hpos, 0);
                    assert_eq!(
                        a.vpos,
                        if line + 1 == expected_lines {
                            0
                        } else {
                            line + 1
                        }
                    );
                }
                assert_eq!(a.lof, interlace && field % 2 == 0);
            }
        }
    }
}

#[test]
fn beam_writes_commit_fields_without_destroying_the_other_vertical_half() {
    let mut a = Agnus::new(AgnusModel::OcsPal8371);
    a.vpos = 0x34;
    a.hpos = 0x40;
    assert!(a.set_vposw(0xffff));
    assert_eq!(a.vpos, 0x34);
    a.step_cck();
    assert_eq!(a.vpos, 0x134);
    assert!(a.lof);
    assert!(a.set_vhposw(0x1234));
    a.step_cck();
    assert_eq!((a.hpos, a.vpos), (0x34, 0x112));
    assert!(a.lof);
    assert!(a.set_vposw(0));
    a.step_cck();
    assert_eq!(a.vpos, 0x12);
    assert!(!a.lof);
}

#[test]
fn noninterlaced_long_field_selection_and_pipelined_readout_use_313_lines() {
    let mut a = Agnus::new(AgnusModel::OcsPal8371);
    a.lof = true;
    a.vpos = 311;
    a.hpos = 225;
    assert_eq!(a.vhposr(), 0x3803); // Lead crosses into line 312, not line zero.
    a.step_cck();
    a.step_cck();
    assert_eq!((a.hpos, a.vpos), (0, 312));
    for _ in 0..227 {
        a.step_cck();
    }
    assert_eq!((a.hpos, a.vpos), (0, 0));
    assert!(a.lof); // LACE controls automatic toggling.
}

#[test]
fn refresh_test_address_and_dummy_copper_fetch_do_not_inject_engine_work() {
    for model in [AgnusModel::OcsPal8371, AgnusModel::OcsNtsc8370] {
        let mut agnus = Agnus::new(model);
        agnus.copper.cop1lc = 0x2000;
        agnus.copper.cop_pc = 0x2040;
        let mut control = agnus.clone();
        assert!(agnus.set_refptr(0xffff));
        assert!(agnus.set_copins(0x0180));
        for _ in 0..8 {
            agnus.step_cck();
            control.step_cck();
        }
        // HRM Appendix A: REFPTR is a DRAM refresh test address; COPINS names
        // a dummy instruction fetch. Neither is a Copper injection interface.
        assert_eq!(agnus, control);
    }
}
