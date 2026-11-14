use denise::{Denise, DeniseModel};

#[test]
fn restored_control_word_retains_ham_ehb_and_seven_plane_distinctions(
) -> Result<(), serde::de::value::Error> {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.color[0] = 0x123;
    denise.color[7] = 0x456;
    denise.color[31] = 0xdef;
    for (word, planes, expected, held) in [
        (0x6800u16, 0x1f, 0xabf, 0xabf),
        (0x6c00, 0x1f, 0xabf, 0xabf),
        (0x7800, 0x3f, 0xdef, 0xabc),
        (0xe800, 0x1f, 0xdef, 0xabc),
        (0xf800, 0x3f, 0xdef, 0xabc),
        (0x6000, 0x3f, 0x677, 0xabc),
        (0x7c00, 0x3f, 0x456, 0xabc),
        (0x0c00, 0x3f, 0x123, 0xabc),
    ] {
        let raw = serde::de::value::U16Deserializer::<serde::de::value::Error>::new(word);
        denise.bplcon0 = serde::Deserialize::deserialize(raw)?;
        assert_eq!(u16::from(denise.bplcon0), word);
        denise.last_ham_rgb = 0xabc;
        assert_eq!(denise.decode_pixel(planes), expected, "BPLCON0 {word:04x}");
        assert_eq!(denise.last_ham_rgb, held, "BPLCON0 {word:04x}");
    }
    Ok(())
}

#[test]
fn decoded_modes_change_only_when_the_register_write_applies() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.sync.hpos = 40;
    denise.color[2] = 0x246;
    denise.color[9] = 0x579;
    denise.color[31] = 0xdef;
    for (value, count, hires, pixel, color) in [
        (0x6800, 6, false, 0x1f, 0xabf),
        (0x2400, 2, false, 2, 0x579),
        (0x2000, 2, false, 2, 0x246),
        (0xe800, 6, true, 0x1f, 0xdef),
        (0x7000, 7, false, 2, 0x246),
    ] {
        let previous = denise.bplcon0;
        assert!(denise.set_bplcon0(value));
        assert_eq!(denise.bplcon0, previous);
        denise.step_cck();
        assert_eq!(denise.bplcon0.value(), value);
        assert_eq!((denise.bplcon0.value() >> 12) & 7, count);
        assert_eq!(denise.is_hires(), hires);
        denise.last_ham_rgb = 0xabc;
        assert_eq!(denise.decode_pixel(pixel), color);
    }
    denise.reset();
    assert_eq!(denise.bplcon0.value(), 0);
    assert_eq!((denise.bplcon0.value() >> 12) & 7, 0);
    assert!(!denise.is_hires());
    assert_eq!(denise.decode_pixel(0x1f), 0);
}

#[test]
fn display_mode_matrix_masks_planes_and_places_pixels_in_both_standard_fields() {
    for ntsc in [false, true] {
        for (lace, odd) in [(false, false), (true, false), (true, true)] {
            for (resolution, maximum) in [(0, 6u16), (0x8000, 4)] {
                for count in 0..=maximum {
                    for dual in [false, true] {
                        for second_first in [false, true] {
                            let mut denise = Denise::new(DeniseModel::Ocs8362);
                            denise.sync.synchronized = true;
                            denise.sync.vertical_blank = false;
                            denise.sync.hpos = 40;
                            denise.sync.vpos = 100;
                            denise.hflop = true;
                            denise.bplcon0 = (resolution
                                | (count << 12)
                                | if dual { 0x400 } else { 0 }
                                | if lace { 4 } else { 0 })
                            .into();
                            denise.bplcon2 = if second_first { 0x40 } else { 0 };
                            denise.sync.hpos = if ntsc { 227 } else { 226 };
                            denise.step_cck();
                            denise.sync.field_odd = odd;
                            denise.strobe_strhor();
                            denise.sync.hpos = 40;
                            denise.sync.vpos = 100;
                            for (index, color) in denise.color.iter_mut().enumerate() {
                                *color = 0x800 | index as u16 * 0x11;
                            }
                            // All physical inputs are high; BPU selects which contribute.
                            for plane in (0..6).rev() {
                                assert!(denise.set_bpldat(plane, 0xffff));
                            }
                            for _ in 0..3 {
                                denise.step_cck();
                            }
                            let expected = if count == 0 {
                                0x800
                            } else if dual {
                                let first = (1u16 << count.div_ceil(2)) - 1;
                                let second = (1u16 << (count / 2)) - 1;
                                let index = if second_first && second != 0 {
                                    8 + second
                                } else {
                                    first
                                };
                                0x800 | index * 0x11
                            } else if count == 6 {
                                0x507 // Half of palette entry 31 ($A0F).
                            } else {
                                0x800 | ((1u16 << count) - 1) * 0x11
                            };
                            let row = if lace { usize::from(odd) } else { 0 };
                            assert_eq!(
                                denise.frame_builder.get_pixel(22, row),
                                denise::frame_builder::rgb444_to_argb32(expected),
                                "NTSC {ntsc}, lace {lace}, odd {odd}, resolution {resolution:x}, count {count}, dual {dual}, second {second_first}"
                            );
                        }
                    }
                }
            }
        }
    }
}
