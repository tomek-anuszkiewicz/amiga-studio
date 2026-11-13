use denise::sprites::Sprites;

fn armed(channels: u8) -> Sprites {
    let mut sprites = Sprites::new();
    for index in 0..8 {
        sprites.write_pos(index, 40);
        sprites.write_data(index, 0x8000);
        if channels & (1 << index) == 0 {
            sprites.write_ctl(index, 0);
        }
    }
    sprites
}

#[test]
fn all_channels_select_their_palette_group_and_lowest_channel_wins() {
    for channel in 0..8 {
        for code in 1..4 {
            let mut s = armed(1 << channel);
            s.write_data(channel, if code & 1 != 0 { 0x8000 } else { 0 });
            s.write_datb(channel, if code & 2 != 0 { 0x8000 } else { 0 });
            let pixel = s.evaluate_pixel(80, 0.into(), &mut 0).unwrap();
            assert_eq!(pixel.color_index, 16 + channel / 2 * 4 + code);
            assert_eq!(pixel.sprite_pair, channel / 2);
        }
    }
    let mut s = armed(0xff);
    let mut collisions = 0;
    assert_eq!(
        s.evaluate_pixel(80, 0xffff.into(), &mut collisions)
            .unwrap()
            .color_index,
        17
    );
    assert_eq!(collisions, 0x7e00);
}

#[test]
fn attachment_combines_four_bits_for_all_pairs_and_preserves_transparency() {
    for pair in 0..4 {
        for code in 0..16 {
            let mut s = armed(3 << (pair * 2));
            let even = pair * 2;
            s.write_ctl(even + 1, 0x80);
            s.write_data(even, if code & 1 != 0 { 0x8000 } else { 0 });
            s.write_datb(even, if code & 2 != 0 { 0x8000 } else { 0 });
            s.write_data(even + 1, if code & 4 != 0 { 0x8000 } else { 0 });
            s.write_datb(even + 1, if code & 8 != 0 { 0x8000 } else { 0 });
            let pixel = s.evaluate_pixel(80, 0.into(), &mut 0);
            assert_eq!(
                pixel.map(|p| p.color_index),
                if code == 0 { None } else { Some(16 + code) }
            );
        }
    }
}

#[test]
fn pair_collisions_follow_odd_enables_even_when_attached_or_hidden() {
    for (first, second, bit) in [
        (0, 1, 9),
        (0, 2, 10),
        (0, 3, 11),
        (1, 2, 12),
        (1, 3, 13),
        (2, 3, 14),
    ] {
        let mut s = armed((1 << (first * 2)) | (1 << (second * 2)));
        let mut collisions = 0;
        s.evaluate_pixel(80, 0.into(), &mut collisions);
        assert_eq!(collisions, 1 << bit);
        let mut s = armed((1 << (first * 2 + 1)) | (1 << (second * 2 + 1)));
        s.write_ctl(first * 2 + 1, 0x80);
        s.write_data(first * 2 + 1, 0x8000);
        s.write_ctl(second * 2 + 1, 0x80);
        s.write_data(second * 2 + 1, 0x8000);
        let mut collisions = 0;
        s.evaluate_pixel(80, 0.into(), &mut collisions);
        assert_eq!(collisions, 0);
        s.evaluate_pixel(
            80,
            ((0x1000 << first) | (0x1000 << second)).into(),
            &mut collisions,
        );
        assert_eq!(collisions, 1 << bit);
    }
}

#[test]
fn sixteen_pixels_repeat_each_line_and_position_data_changes_allow_same_line_reuse() {
    let mut s = armed(1);
    s.write_data(0, 0xa001);
    let mut collisions = 0;
    assert!(s.evaluate_pixel(79, 0.into(), &mut collisions).is_none());
    let pixels: Vec<_> = (80..97)
        .map(|x| s.evaluate_pixel(x, 0.into(), &mut collisions).is_some())
        .collect();
    assert_eq!(
        pixels,
        [
            true, false, true, false, false, false, false, false, false, false, false, false,
            false, false, false, true, false
        ]
    );
    s.write_pos(0, 60);
    s.write_data(0, 0x8000);
    s.write_datb(0, 0x8000);
    assert_eq!(
        s.evaluate_pixel(120, 0.into(), &mut collisions)
            .unwrap()
            .color_index,
        19
    );
    s.step_cck(0);
    assert_eq!(
        s.evaluate_pixel(120, 0.into(), &mut collisions)
            .unwrap()
            .color_index,
        19
    );
    s.write_ctl(0, s.channels()[0].ctl);
    s.step_cck(0);
    assert!(s.evaluate_pixel(120, 0.into(), &mut collisions).is_none());
}

#[test]
fn idle_evaluation_clears_pair_presence_preserves_collisions_and_can_arm_any_channel() {
    for channel in 0..8 {
        let mut s = armed(0);
        s.write_data(channel, 0xffff);
        s.evaluate_pixel(80, 0.into(), &mut 0);
        s.write_data(channel, 0x8000);
        s.write_ctl(channel, 0);
        s.step_cck(0);
        s.collision_pairs = 0x0f;
        let channels = *s.channels();
        let mut collisions = 0x7e00;
        assert!(s
            .evaluate_pixel(79, 0xffff.into(), &mut collisions)
            .is_none());
        assert_eq!(s.collision_pairs, 0);
        assert_eq!(collisions, 0x7e00);
        assert_eq!(*s.channels(), channels);

        s.write_data(channel, 0x8000);
        let pixel = s
            .evaluate_pixel(80, 0xffff.into(), &mut collisions)
            .unwrap();
        assert_eq!(pixel.color_index, 17 + channel / 2 * 4);
        assert_eq!(s.channels()[channel].pixel_counter, 15);
        assert_eq!(s.collision_pairs, 1 << (channel / 2));
        assert_eq!(collisions, 0x7e00);
    }
}

#[test]
fn disarmed_serializer_finishes_both_visible_and_transparent_words() {
    for data in [0xa001, 0] {
        let mut s = armed(1);
        s.write_data(0, data);
        let mut collisions = 0;
        s.evaluate_pixel(80, 0.into(), &mut collisions);
        s.write_ctl(0, s.channels()[0].ctl);
        for pixel in 1..16 {
            let output = s.evaluate_pixel(80 + pixel, 0.into(), &mut collisions);
            assert_eq!(output.is_some(), data & (0x8000 >> pixel) != 0);
            assert_eq!(s.channels()[0].pixel_counter, 15 - pixel as u8);
            assert_eq!(
                s.channels()[0].shift_a,
                (u32::from(data) << (pixel + 1)) as u16
            );
        }
        assert!(s.evaluate_pixel(96, 0.into(), &mut collisions).is_none());
        assert_eq!(s.collision_pairs, 0);
        assert_eq!(s.channels()[0].pixel_counter, 0);
    }
}

#[test]
fn independent_pair_colors_cover_both_channels_and_all_transparent_combinations() {
    for pair in 0..4 {
        for even in 0..4 {
            for odd in 0..4 {
                let mut s = armed(3 << (pair * 2));
                for (channel, code) in [(pair * 2, even), (pair * 2 + 1, odd)] {
                    s.write_data(channel, if code & 1 != 0 { 0x8000 } else { 0 });
                    s.write_datb(channel, if code & 2 != 0 { 0x8000 } else { 0 });
                }
                let code = if even != 0 { even } else { odd };
                let actual = s.evaluate_pixel(80, 0.into(), &mut 0);
                assert_eq!(
                    actual.map(|p| p.color_index),
                    if code == 0 {
                        None
                    } else {
                        Some(16 + pair * 4 + code)
                    }
                );
            }
        }
    }
}

#[test]
fn all_collision_presence_masks_follow_odd_enables_independently_of_attachment() {
    for attached in [false, true] {
        for active in 0..16u8 {
            for enabled in 0..16u16 {
                let mut s = armed(0);
                for pair in 0..4 {
                    let odd = pair * 2 + 1;
                    s.write_ctl(odd, if attached { 0x80 } else { 0 });
                    if active & (1 << pair) != 0 {
                        s.write_data(odd, 0x8000);
                    }
                }
                let presence = active & enabled as u8;
                let mut expected = 0x8001;
                for (first, second, bit) in [
                    (0, 1, 9),
                    (0, 2, 10),
                    (0, 3, 11),
                    (1, 2, 12),
                    (1, 3, 13),
                    (2, 3, 14),
                ] {
                    if presence & (1 << first) != 0 && presence & (1 << second) != 0 {
                        expected |= 1 << bit;
                    }
                }
                let mut collisions = 0x8001;
                s.evaluate_pixel(80, (enabled << 12).into(), &mut collisions);
                assert_eq!(s.collision_pairs, presence);
                assert_eq!(collisions, expected);
                s.reset();
                assert_eq!(s, Sprites::new());
                assert!(s
                    .evaluate_pixel(80, 0xffff.into(), &mut collisions)
                    .is_none());
            }
        }
    }
}
