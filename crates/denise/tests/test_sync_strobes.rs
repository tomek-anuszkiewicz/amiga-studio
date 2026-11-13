use config::DeniseModel;
use denise::Denise;

#[test]
fn repeated_address_strobes_update_receiver_state_immediately() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.sync.hpos = 90;
    denise.sync.vpos = 77;
    denise.strobe_strequ();
    assert_eq!(denise.sync.hpos, 0);
    assert_eq!(denise.sync.vpos, 0);
    assert!(denise.sync.vertical_blank && denise.sync.equalization);
    denise.strobe_strequ();
    assert_eq!(denise.sync.vpos, 1, "the same address can be another event");
    denise.strobe_strvbl();
    assert!(denise.sync.vertical_blank && !denise.sync.equalization);
    denise.strobe_strlong();
    assert!(denise.sync.long_line);
    denise.strobe_strhor();
    assert!(!denise.sync.vertical_blank && !denise.sync.long_line);
    assert!(denise.mutations.iter().all(Option::is_none));
    denise.reset();
    assert!(!denise.sync.synchronized);
}

#[test]
fn field_boundaries_follow_received_strobes_for_both_standards_and_interlace() {
    for base_lines in [262u16, 312] {
        for interlaced in [false, true] {
            let mut denise = Denise::new(DeniseModel::Ocs8362);
            denise.bplcon0 = if interlaced { 4 } else { 0 };
            // Joining a field in progress must not publish a short frame.
            for _ in 0..20 {
                denise.strobe_strhor();
            }
            denise.strobe_strequ();
            assert_eq!(denise.frame_builder.frame_number, 0);
            for field in 0..4 {
                let odd = interlaced && field % 2 != 0;
                let lines = base_lines + u16::from(odd);
                for line in 1..lines {
                    for _ in denise.sync.hpos..227 {
                        denise.step_cck();
                    }
                    if line < 8 {
                        denise.strobe_strequ();
                    } else if line < 26 {
                        denise.strobe_strvbl();
                    } else {
                        denise.strobe_strhor();
                    }
                    assert_eq!(denise.sync.vpos, line);
                    assert_eq!(denise.sync.field_odd, odd);
                    denise.strobe_strlong();
                    assert_eq!(denise.sync.vpos, line);
                }
                for _ in denise.sync.hpos..227 {
                    denise.step_cck();
                }
                denise.strobe_strequ();
                assert_eq!(denise.sync.vpos, 0);
                assert_eq!(denise.sync.field_lines, lines);
                assert_eq!(denise.frame_builder.frame_number, field);
                for _ in 0..18 {
                    denise.step_cck();
                }
                assert_eq!(denise.frame_builder.frame_number, field);
                denise.step_cck();
                assert_eq!(denise.frame_builder.frame_number, field + 1);
                assert_eq!(
                    denise.frame_builder.height,
                    if interlaced {
                        if field > 0 {
                            u32::from(base_lines - 26) * 2
                        } else {
                            u32::from(base_lines - 26 - 1) * 2 - 1
                        }
                    } else {
                        u32::from(base_lines - 26 - 1) * 2
                    }
                );
            }
            denise.reset();
            assert!(!denise.sync.field_locked);
            assert_eq!(denise.frame_builder.frame_number, 0);
        }
    }
}
