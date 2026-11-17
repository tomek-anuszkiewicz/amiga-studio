use config::DeniseModel;
use denise::{frame_builder, Denise};

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
