use config::DeniseModel;
use denise::{frame_builder, Denise};

#[test]
fn bitplane_words_cross_the_line_strobe_and_the_next_row_starts_with_new_data() {
    for line_ccks in [227, 228] {
        for hires in [false, true] {
            for scroll in [0, 3] {
                let mut d = Denise::new(DeniseModel::Ocs8362);
                d.diwstrt = 0x1b51;
                d.diwstop = 0x37d1;
                d.bplcon0 = (if hires { 0x9000 } else { 0x1000 }).into();
                d.bplcon1 = scroll;
                d.color[0] = 0x00f;
                d.color[1] = 0x0f0;
                d.strobe_strhor();
                d.sync.vpos = 28;
                for hpos in 0..line_ccks {
                    if hpos == 224 {
                        assert!(d.set_bpldat(0, 0xb6d3));
                    }
                    d.step_cck();
                }
                d.strobe_strhor();
                for hpos in 0..62 {
                    if hpos == 49 {
                        assert!(!d.bpl_armed);
                        assert!(d.set_bpldat(0, 0x593a));
                    }
                    d.step_cck();
                }

                // Each word starts shifting one CCK after issue, then traverses
                // the two-sample output delay. The stored row starts at CCK 49.
                let samples_per_bit = if hires { 1 } else { 2 };
                for (row, cols, first_sample, word) in
                    [(0, 696usize..720, 706, 0xb6d3u16), (2, 0..48, 6, 0x593a)]
                {
                    let start = first_sample + usize::from(scroll) * samples_per_bit;
                    for x in cols {
                        let set = x.checked_sub(start).is_some_and(|offset| {
                            let bit = offset / samples_per_bit;
                            bit < 16 && word & (0x8000 >> bit) != 0
                        });
                        let expected = if set { 0xff00ff00 } else { 0xff0000ff };
                        for y in row..row + 2 {
                            assert_eq!(
                                d.frame_builder.get_pixel(x, y),
                                expected,
                                "CCKs={line_ccks}, hires={hires}, scroll={scroll}, x={x}, y={y}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn ham_modifiers_keep_the_held_color_on_visible_tails_and_field_blanking_clears_them() {
    for interlaced in [false, true] {
        for field_end in [false, true] {
            let mut d = Denise::new(DeniseModel::Ocs8362);
            d.bplcon0 = (0x6800 | if interlaced { 4 } else { 0 }).into();
            d.diwstrt = 0x1b51;
            d.diwstop = 0x37d1;
            d.color[0] = 0x014;
            d.color[1] = 0xf97;
            d.strobe_strequ();
            d.strobe_strhor();
            d.sync.vpos = if field_end { 311 } else { 28 };
            let y = usize::from(d.sync.vpos - 28) * 2;
            for hpos in 0..227 {
                if hpos == 224 {
                    // Palette color 1, followed by fifteen blue-component writes of 2.
                    assert!(d.set_bpldat(4, 0x7fff));
                    assert!(d.set_bpldat(1, 0x7fff));
                    assert!(d.set_bpldat(0, 0x8000));
                }
                d.step_cck();
            }
            if field_end {
                d.strobe_strequ();
            } else {
                d.strobe_strhor();
            }
            for _ in 0..20 {
                d.step_cck();
            }
            for x in 706..720 {
                let expected = match x {
                    706..708 => 0xffff9977,
                    712..720 if field_end => 0xff000000,
                    _ => 0xffff9922,
                };
                assert_eq!(d.frame_builder.get_pixel(x, y), expected, "x={x}, y={y}");
                let other_row = if interlaced { 0xff000000 } else { expected };
                assert_eq!(d.frame_builder.get_pixel(x, y + 1), other_row);
            }
        }
    }
}

#[test]
fn first_ham_modifier_after_horizontal_blanking_uses_color_zero() {
    let mut d = Denise::new(DeniseModel::Ocs8362);
    d.bplcon0 = 0x6800.into();
    // The first pixel inside this window is already a HAM modifier.
    d.diwstrt = 0x1b65;
    d.diwstop = 0x37c1;
    d.color[0] = 0x014;
    d.color[1] = 0xf97;
    d.strobe_strequ();
    for _ in 0..28 {
        d.strobe_strhor();
    }
    for hpos in 0..227 {
        if hpos == 220 {
            // Select COLOR01, then set its blue component to 2.
            assert!(d.set_bpldat(4, 0x7fff));
            assert!(d.set_bpldat(1, 0x7fff));
            assert!(d.set_bpldat(0, 0x8000));
        }
        d.step_cck();
    }

    d.strobe_strhor();
    for hpos in 0..=50 {
        if hpos == 48 {
            // Set blue to 2 from the first bit of the new word.
            assert!(d.set_bpldat(4, 0xffff));
            assert!(d.set_bpldat(1, 0xffff));
            assert!(d.set_bpldat(0, 0));
        }
        d.step_cck();
    }

    // Blanking resets the held color to COLOR00. Blue 2 yields RGB $012.
    for x in 2..=3 {
        assert_eq!(d.frame_builder.get_pixel(x, 2), 0xff001122, "x={x}, y=2");
    }
}

#[test]
fn manual_bitplane_data_committed_before_horizontal_blanking_keeps_shifting() {
    let mut d = Denise::new(DeniseModel::Ocs8362);
    d.bplcon0 = 0x1000.into();
    d.strobe_strhor();
    for hpos in 0..18 {
        if hpos == 17 {
            assert!(d.set_bpldat(0, 0x96a5));
        }
        d.step_cck();
    }
    assert_eq!(d.shifters[0], 0x96a5);
    d.step_cck();
    assert!(d.bpl_armed);
    assert_eq!(d.shifters[0], 0x5a94);
    assert_eq!(d.scroll_history[0], 0b10);
}

#[test]
fn standard_window_keeps_first_and_last_word_samples_after_bpl1dat_commit() {
    for hires in [false, true] {
        for scroll in [0, 3] {
            let mut d = Denise::new(DeniseModel::Ocs8362);
            d.diwstrt = 0x0081;
            d.diwstop = 0x00c1;
            d.bplcon0 = (if hires { 0x9000 } else { 0x1000 }).into();
            d.bplcon1 = scroll;
            d.color[1] = 0x0f0;
            let fetch_period = if hires { 4 } else { 8 };
            let words = if hires { 40 } else { 20 };
            for line in 0..2 {
                d.strobe_strhor();
                d.sync.vpos = 28 + line as u16;
                for hpos in 0..227 {
                    if hpos >= 62
                        && (hpos - 62) % fetch_period == 0
                        && (hpos - 62) / fetch_period < words
                    {
                        assert!(d.set_bpldat(0, 0xc003));
                    }
                    d.step_cck();
                }
                for x in 56..700 {
                    let bit = (x - 58isize) / if hires { 1 } else { 2 } - scroll as isize;
                    let foreground = (58..698).contains(&x)
                        && bit >= 0
                        && (0xc003u16 & (0x8000 >> (bit % 16))) != 0;
                    assert_eq!(
                        d.frame_builder.get_pixel(x as usize, line * 2),
                        if foreground { 0xff00ff00 } else { 0xff000000 },
                        "hires={hires}, scroll={scroll}, line={line}, x={x}"
                    );
                }
            }
        }
    }
}

#[test]
fn horizontal_window_matches_receiver_origin_on_consecutive_lines() {
    // The receiver restarts at count 2. Comparing the delayed output phase
    // must still admit that first count, for both even and odd starts.
    for hires in [false, true] {
        for start in [2, 3] {
            let mut d = Denise::new(DeniseModel::Ocs8362);
            d.diwstrt = start;
            d.diwstop = 0xc0;
            d.bplcon0 = (if hires { 0x9000 } else { 0x1000 }).into();
            d.color[1] = 0x0f0;
            for line in 0..2 {
                d.strobe_strhor();
                d.sync.vpos = 28 + line as u16;
                for hpos in 0..227 {
                    if hpos > 0 {
                        d.bpl_armed = true;
                        d.shifters[0] = 0xffff;
                    }
                    d.step_cck();
                }
                assert_eq!(
                    d.frame_builder.get_pixel(60, line * 2),
                    0xff00ff00,
                    "hires={hires}, start={start}, line={line}"
                );
                assert_eq!(d.frame_builder.get_pixel(696, line * 2), 0xff000000);
            }
        }
    }
}

#[test]
fn horizontal_window_preserves_h0_at_both_edges_on_consecutive_lines() {
    // HRM Appendix A exposes H0 in both DIW registers. A low-resolution
    // counter period covers two hires samples, including an even edge.
    for hires in [false, true] {
        for start in [0x80, 0x81] {
            for stop in [0xc0, 0xc1] {
                for scroll in [0, 3] {
                    let mut d = Denise::new(DeniseModel::Ocs8362);
                    d.diwstrt = start;
                    d.diwstop = stop;
                    d.bplcon0 = (if hires { 0x9000 } else { 0x1000 }).into();
                    d.bplcon1 = scroll;
                    d.color[1] = 0x0f0;
                    for line in 0..2 {
                        d.strobe_strhor();
                        d.sync.vpos = 28 + line as u16;
                        for hpos in 0..227 {
                            // Continuous foreground, with enough preceding bits
                            // to fill nonzero scroll before either DIW edge.
                            if hpos > 0 {
                                d.bpl_armed = true;
                                d.shifters[0] = 0xffff;
                            }
                            d.step_cck();
                        }
                        let first = usize::from(start) * 2 - 200;
                        let end = usize::from(stop | 0x100) * 2 - 200;
                        for x in first - 2..end + 2 {
                            let expected = if (first..end).contains(&x) {
                                0xff00ff00
                            } else {
                                0xff000000
                            };
                            assert_eq!(d.frame_builder.get_pixel(x, line * 2), expected,
                                "hires={hires}, start={start:x}, stop={stop:x}, scroll={scroll}, line={line}, x={x}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_denise_display_window_uses_horizontal_bits_only() {
    let mut inside_vertical_window = Denise::new(DeniseModel::Ocs8362);
    let mut outside_vertical_window = Denise::new(DeniseModel::Ocs8362);

    for (denise, diwstrt) in [
        (&mut inside_vertical_window, 0x6481),
        (&mut outside_vertical_window, 0xAA81),
    ] {
        denise.diwstrt = diwstrt;
        denise.diwstop = 0xC8C1;
        denise.bplcon0 = (0x1000).into();
        denise.color[1] = 0x00F0;

        denise.sync.vpos = 150;
        denise.sync.vertical_blank = false;
        denise.strobe_strhor();
        denise.sync.vpos = 150;
        for hpos in 0..80 {
            if hpos == 64 {
                assert!(denise.set_bpldat(0, 0xFFFF));
            }
            denise.step_cck();
        }
    }

    let color = frame_builder::rgb444_to_argb32(0x00F0);
    let green_pixels = (60..124)
        .filter(|&x| inside_vertical_window.frame_builder.get_pixel(x, 244) == color)
        .count();
    assert!(green_pixels > 0);
    for x in 60..124 {
        assert_eq!(
            outside_vertical_window.frame_builder.get_pixel(x, 244),
            inside_vertical_window.frame_builder.get_pixel(x, 244),
            "Denise must ignore DIW vertical bits at pixel {x}"
        );
    }
}
