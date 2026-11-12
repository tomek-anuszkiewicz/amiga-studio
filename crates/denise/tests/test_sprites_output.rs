use denise::sprites::Sprites;

fn armed(channels: u8) -> Sprites {
    let mut sprites = Sprites::new();
    for (index, channel) in sprites.channels.iter_mut().enumerate() {
        channel.pos = 40;
        channel.data_a = 0x8000;
        channel.is_armed = channels & (1 << index) != 0;
    }
    sprites
}

#[test]
fn all_channels_select_their_palette_group_and_lowest_channel_wins() {
    for channel in 0..8 {
        for code in 1..4 {
            let mut s = armed(1 << channel);
            s.channels[channel].data_a = if code & 1 != 0 { 0x8000 } else { 0 };
            s.channels[channel].data_b = if code & 2 != 0 { 0x8000 } else { 0 };
            let pixel = s.evaluate_pixel(80, 0, &mut 0).unwrap();
            assert_eq!(pixel.color_index, 16 + channel / 2 * 4 + code);
            assert_eq!(pixel.sprite_pair, channel / 2);
        }
    }
    let mut s = armed(0xff);
    let mut collisions = 0;
    assert_eq!(
        s.evaluate_pixel(80, 0xffff, &mut collisions)
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
            s.channels[even + 1].ctl = 0x80;
            s.channels[even].data_a = if code & 1 != 0 { 0x8000 } else { 0 };
            s.channels[even].data_b = if code & 2 != 0 { 0x8000 } else { 0 };
            s.channels[even + 1].data_a = if code & 4 != 0 { 0x8000 } else { 0 };
            s.channels[even + 1].data_b = if code & 8 != 0 { 0x8000 } else { 0 };
            let pixel = s.evaluate_pixel(80, 0, &mut 0);
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
        s.evaluate_pixel(80, 0, &mut collisions);
        assert_eq!(collisions, 1 << bit);
        let mut s = armed((1 << (first * 2 + 1)) | (1 << (second * 2 + 1)));
        s.channels[first * 2 + 1].ctl = 0x80;
        s.channels[second * 2 + 1].ctl = 0x80;
        let mut collisions = 0;
        s.evaluate_pixel(80, 0, &mut collisions);
        assert_eq!(collisions, 0);
        s.evaluate_pixel(80, (0x1000 << first) | (0x1000 << second), &mut collisions);
        assert_eq!(collisions, 1 << bit);
    }
}

#[test]
fn sixteen_pixels_repeat_each_line_and_position_data_changes_allow_same_line_reuse() {
    let mut s = armed(1);
    s.channels[0].data_a = 0xa001;
    let mut collisions = 0;
    assert!(s.evaluate_pixel(79, 0, &mut collisions).is_none());
    let pixels: Vec<_> = (80..97)
        .map(|x| s.evaluate_pixel(x, 0, &mut collisions).is_some())
        .collect();
    assert_eq!(
        pixels,
        [
            true, false, true, false, false, false, false, false, false, false, false, false,
            false, false, false, true, false
        ]
    );
    s.channels[0].pos = 60;
    s.channels[0].data_a = 0x8000;
    s.channels[0].data_b = 0x8000;
    assert_eq!(
        s.evaluate_pixel(120, 0, &mut collisions)
            .unwrap()
            .color_index,
        19
    );
    s.step_cck(0);
    assert_eq!(
        s.evaluate_pixel(120, 0, &mut collisions)
            .unwrap()
            .color_index,
        19
    );
    s.channels[0].is_armed = false;
    s.step_cck(0);
    assert!(s.evaluate_pixel(120, 0, &mut collisions).is_none());
}
