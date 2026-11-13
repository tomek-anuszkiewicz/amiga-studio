use denise::{decode_dual_playfield, decode_ehb, decode_ham6, Denise, DeniseModel};

fn visible() -> Denise {
    let mut d = Denise::new(DeniseModel::Ocs8362);
    d.sync.synchronized = true;
    d.sync.vertical_blank = false;
    d.sync.hpos = 40;
    d.sync.vpos = 100;
    d.frame_builder.begin_line(false, false, false, false);
    d.hflop = true;
    d.diwstrt = 0x0081;
    d.diwstop = 0x00c1;
    d
}

#[test]
fn all_plane_collision_enable_and_match_values_use_current_serialized_bits() {
    let mut d = visible();
    d.bplcon0 = 0x6000;
    for sprite in [0, 2, 4, 6] {
        d.sprites.write_pos(sprite, 40);
        d.sprites.write_data(sprite, 0xffff);
    }
    for control in 0..0x1000u16 {
        d.clxcon = control.into();
        for planes in 0..64u8 {
            d.sync.hpos = 40;
            d.hflop = true;
            d.bpl_armed = true;
            d.sprites.step_cck(0);
            d.clxdat = 0;
            for plane in 0..6 {
                d.shifters[plane] = if planes & (1 << plane) != 0 {
                    0xffff
                } else {
                    0
                };
            }
            d.step_cck();
            let mismatch = (u16::from(planes) ^ control) & (control >> 6);
            let odd = mismatch & 0x15 == 0;
            let even = mismatch & 0x2a == 0;
            let expected = 0x7e00
                | if odd { 0x1e } else { 0 }
                | if even { 0x1e0 } else { 0 }
                | u16::from(odd && even);
            assert_eq!(
                d.read_clxdat(),
                expected,
                "control={control:04x} planes={planes:02x}"
            );
        }
    }
}

#[test]
fn mid_line_sprite_commits_arm_reload_and_disarm_without_stopping_serialization() {
    let mut d = visible();
    d.step_cck();
    assert_eq!(d.sprites.channels()[0].pixel_counter, 0);
    assert!(d.set_sprpos(0, 42));
    assert!(d.set_sprdata(0, 0xffff));
    d.step_cck();
    assert!(d.sprites.channels()[0].is_armed);
    assert_eq!(d.sprites.channels()[0].pixel_counter, 0);
    d.step_cck();
    assert_eq!(d.sprites.channels()[0].pixel_counter, 14);

    assert!(d.set_sprctl(0, 0));
    d.step_cck();
    assert!(!d.sprites.channels()[0].is_armed);
    assert_eq!(d.sprites.channels()[0].pixel_counter, 12);
    assert!(d.set_sprpos(0, 46));
    d.step_cck();
    assert_eq!(d.sprites.channels()[0].pixel_counter, 10);
    assert!(d.set_sprdata(0, 0));
    d.step_cck();
    assert_eq!(d.sprites.channels()[0].pixel_counter, 8);
    d.step_cck();
    assert_eq!(d.sprites.channels()[0].pixel_counter, 14);
    assert_eq!(d.sprites.channels()[0].shift_a, 0);
    assert!(d.set_sprctl(0, 0));
    d.step_cck();
    assert!(!d.sprites.channels()[0].is_armed);
    for remaining in (0..=10).step_by(2).rev() {
        d.step_cck();
        assert_eq!(d.sprites.channels()[0].pixel_counter, remaining);
    }
    d.step_cck();
    assert_eq!(d.sprites.collision_pairs, 0);
}

#[test]
fn palette_dual_ham_and_half_brite_match_book_color_rules() {
    let mut palette = [0; 32];
    for (index, color) in palette.iter_mut().enumerate() {
        *color = index as u16;
    }
    for first in 0..8u8 {
        for second in 0..8u8 {
            let data = (first & 1)
                | ((second & 1) << 1)
                | ((first & 2) << 1)
                | ((second & 2) << 2)
                | ((first & 4) << 2)
                | ((second & 4) << 3);
            assert_eq!(
                decode_dual_playfield(data, &palette, false),
                if first != 0 {
                    u16::from(first)
                } else if second != 0 {
                    8 + u16::from(second)
                } else {
                    0
                }
            );
            assert_eq!(
                decode_dual_playfield(data, &palette, true),
                if second != 0 {
                    8 + u16::from(second)
                } else {
                    u16::from(first)
                }
            );
        }
    }
    palette[5] = 0xf97;
    assert_eq!(decode_ehb(5, &palette), 0xf97);
    assert_eq!(decode_ehb(0x25, &palette), 0x743);
    let mut held = 0;
    for (code, expected) in [(5, 0xf97), (0x12, 0xf92), (0x23, 0x392), (0x34, 0x342)] {
        assert_eq!(decode_ham6(code, &palette, &mut held), expected);
    }
}

#[test]
fn bpl1_commit_loads_serializers_and_word_exhaustion_does_not_reload_old_latches() {
    let mut d = visible();
    d.bplcon0 = 0x2000;
    d.set_bpldat(1, 0xffff);
    d.step_cck();
    assert_eq!(d.shifters, [0; 6]);
    d.set_bpldat(0, 0xffff);
    d.step_cck();
    assert_eq!(d.shifters, [0xffff, 0xffff, 0, 0, 0, 0]);
    for _ in 0..16 {
        d.step_cck();
    }
    assert_eq!(d.shifters, [0; 6]);
    assert_eq!(d.bpldat[0], 0xffff);
    assert_eq!(d.bpldat[1], 0xffff);
}

#[test]
fn odd_and_even_scroll_delays_cover_all_values_and_cross_word_boundaries() {
    for odd in 0..16 {
        for even in 0..16 {
            let mut d = visible();
            d.bplcon1 = odd | (even << 4);
            d.shifters = [0x8000, 0x8000, 0, 0, 0, 0];
            for position in 0..32 {
                if position == 16 {
                    d.shifters = [0x8000, 0x8000, 0, 0, 0, 0];
                }
                let expected = u8::from(position == odd || position == odd + 16)
                    | (u8::from(position == even || position == even + 16) << 1);
                assert_eq!(
                    d.shift_pixel(),
                    expected,
                    "scroll {odd}/{even}, position {position}"
                );
            }
        }
    }
}

#[test]
fn supported_normal_resolutions_and_plane_counts_render_independent_palette_values() {
    for (resolution, maximum) in [(0, 5), (0x8000, 4)] {
        for count in 1..=maximum {
            let mut d = visible();
            d.set_bplcon0(resolution | (count << 12));
            let index = (1 << count) - 1;
            d.set_color(index, 0x0f00);
            for plane in (0..count as usize).rev() {
                d.set_bpldat(plane, 0x8000);
            }
            for _ in 0..3 {
                d.step_cck();
            }
            assert_eq!(d.frame_builder.get_pixel(22, 0), 0xffff0000);
            assert_eq!(
                d.frame_builder.get_pixel(23, 0),
                if resolution == 0 {
                    0xffff0000
                } else {
                    0xff000000
                }
            );
            assert_eq!(d.frame_builder.get_pixel(24, 0), 0xff000000);
        }
    }
}

#[test]
fn special_modes_render_book_colors_in_both_interlaced_fields() {
    // Four successive source codes, with expectations independent of decoder helpers.
    for (mode, codes, expected) in [
        (
            0x6800,
            [5u8, 0x12, 0x23, 0x34],
            [0xf97u16, 0xf92, 0x392, 0x342],
        ),
        (0x6000, [5, 0x25, 5, 0x25], [0xf97, 0x743, 0xf97, 0x743]),
        (0x6400, [1, 2, 3, 0], [0xf00, 0x0f0, 0xf00, 0]),
        (0xc400, [1, 2, 3, 0], [0xf00, 0x0f0, 0xf00, 0]),
    ] {
        for ntsc in [false, true] {
            for (lace, odd) in [(false, false), (true, false), (true, true)] {
                let mut d = visible();
                d.bplcon0 = mode | if lace { 4 } else { 0 };
                d.color[1] = 0xf00;
                d.color[5] = 0xf97;
                d.color[9] = 0x0f0;
                d.frame_builder.begin_line(false, false, false, true);
                d.frame_builder.step_cck(if ntsc { 227 } else { 226 }, 99);
                d.frame_builder.begin_line(lace, odd, false, false);
                for plane in (0..6).rev() {
                    let word = codes.iter().enumerate().fold(0, |word, (pixel, code)| {
                        word | (u16::from((code >> plane) & 1) << (15 - pixel))
                    });
                    assert!(d.set_bpldat(plane, word));
                }
                for _ in 0..4 {
                    d.step_cck();
                }
                let row = usize::from(lace && odd);
                let stride = if mode & 0x8000 == 0 { 2 } else { 1 };
                for (pixel, expected) in expected.iter().enumerate() {
                    assert_eq!(
                        d.frame_builder.get_pixel(22 + pixel * stride, row),
                        denise::frame_builder::rgb444_to_argb32(*expected),
                        "mode {mode:x}, lace {lace}, odd {odd}, pixel {pixel}"
                    );
                }
            }
        }
    }
}

#[test]
fn palette_and_resolution_changes_take_effect_during_the_line() {
    let mut d = visible();
    d.set_bplcon0(0x1000);
    d.set_color(1, 0x0f00);
    d.set_bpldat(0, 0xffff);
    for _ in 0..4 {
        d.step_cck();
    }
    d.set_color(1, 0x00f0);
    d.set_bplcon0(0x9000);
    for _ in 0..3 {
        d.step_cck();
    }
    assert_eq!(d.frame_builder.get_pixel(22, 0), 0xffff0000);
    assert_eq!(d.frame_builder.get_pixel(38, 0), 0xff00ff00);
    assert!(d.is_hires());
}

#[test]
fn normal_playfield_uses_pf2_priority_and_hidden_sprites_still_collide() {
    for priority in 0..=4 {
        for pair in 0..4 {
            let mut d = visible();
            d.bplcon0 = 0x1000;
            d.bplcon2 = priority << 3;
            d.bpl_armed = true;
            d.shifters[0] = 0xffff;
            d.color[1] = 0x0f00;
            d.color[17 + pair * 4] = 0x00f0;
            d.sprites.write_pos(pair * 2, 42);
            d.sprites.write_data(pair * 2, 0x8000);
            for _ in 0..4 {
                d.step_cck();
            }
            assert_eq!(
                d.frame_builder.get_pixel(26, 0),
                if pair < priority as usize {
                    0xff00ff00
                } else {
                    0xffff0000
                }
            );
            assert_eq!(d.clxdat & (0x22 << pair), 0x22 << pair);
        }
    }
}

#[test]
fn all_collision_groups_use_enable_and_match_values_before_visual_priority() {
    for (planes, control, expected) in [
        (0x15, 0xf000, 0x7fff),
        (0x15, 0xffff, 0x7e1e),
        (0x15, 0xffc0, 0x7fe0),
        (0x0f, 0xffff, 0x7e00),
    ] {
        let mut d = visible();
        d.bplcon0 = 0x6000;
        d.bpl_armed = true;
        d.clxcon = control.into();
        for plane in 0..6 {
            d.shifters[plane] = if planes & (1 << plane) != 0 {
                0xffff
            } else {
                0
            };
        }
        for sprite in 0..8 {
            d.sprites.write_pos(sprite, 40);
            d.sprites.write_data(sprite, 0x8000);
        }
        d.step_cck();
        assert_eq!(d.clxdat_debug(), expected);
        assert_eq!(d.read_clxdat(), expected);
        assert_eq!(d.clxdat_debug(), 0);
    }
}

#[test]
fn dual_playfield_priority_retains_the_books_invisible_sprite_boundary() {
    // HRM 7, Table 7-2 example: PF2PRI=1, PF2P=2, PF1P=0.
    // PF1 hides sprite 0 even where the frontmost PF2 visually covers PF1.
    for (planes, expected) in [(0, 0x0fff), (1, 0x0f00), (2, 0x0fff), (3, 0x00f0)] {
        let mut d = visible();
        d.bplcon0 = 0x2400;
        d.bplcon2 = 0x0050;
        d.bpl_armed = true;
        d.color[1] = 0x0f00;
        d.color[9] = 0x00f0;
        d.color[17] = 0x0fff;
        for plane in 0..2 {
            d.shifters[plane] = if planes & (1 << plane) != 0 {
                0xffff
            } else {
                0
            };
        }
        d.sprites.write_pos(0, 42);
        d.sprites.write_data(0, 0x8000);
        for _ in 0..4 {
            d.step_cck();
        }
        assert_eq!(
            d.frame_builder.get_pixel(26, 0),
            denise::frame_builder::rgb444_to_argb32(expected),
            "playfield occupancy {planes}"
        );
    }
}

#[test]
fn blanking_skips_framebuffer_writes_while_registers_and_receiver_clocks_continue() {
    for vertical in [false, true] {
        let mut d = visible();
        d.color[0] = 0xf00;
        d.step_cck();
        let before = d.frame_builder.frame_buffer().to_vec();
        d.sync.hpos = if vertical { 40 } else { 18 };
        d.sync.vertical_blank = vertical;
        d.set_color(0, 0x0f0);
        for _ in 0..18 {
            d.step_cck();
        }
        assert_eq!(d.frame_builder.frame_buffer(), before);
        assert_eq!(d.color[0], 0x0f0);
        assert_eq!(d.sync.hpos, if vertical { 58 } else { 36 });
    }
}
