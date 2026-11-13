use denise::{Denise, DeniseModel};

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
                            denise.bplcon0 = resolution
                                | (count << 12)
                                | if dual { 0x400 } else { 0 }
                                | if lace { 4 } else { 0 };
                            denise.bplcon2 = if second_first { 0x40 } else { 0 };
                            denise
                                .frame_builder
                                .step_cck(if ntsc { 227 } else { 226 }, 99);
                            denise.frame_builder.begin_line(lace, odd, false, false);
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
