use config::BeamPosition;
use sprites::{SpritePixel, Sprites};

#[inline]
fn beam(vpos: u16) -> BeamPosition {
    BeamPosition {
        hpos: 0,
        vpos,
        lof: false,
    }
}

#[test]
fn test_sprite_decoding_and_reset() {
    let mut sprites = Sprites::new();
    // Sprite 0:
    // POS: VSTART[7..0] = 0x64 (100), HSTART[8..1] = 0x32 (50)
    // CTL: VSTOP[7..0] = 0x78 (120), ATTACH = 1 (bit 7), SV8 = 1 (bit 2), EV8 = 1 (bit 1), SH0 = 1 (bit 0)
    let pos = (0x64 << 8) | 0x32;
    let ctl = (0x78 << 8) | 0x80 | (1 << 2) | (1 << 1) | 1;

    sprites.set_pos(0, pos);
    sprites.set_ctl(0, ctl);

    let ch0 = &sprites.channels[0];
    assert_eq!(ch0.vstart(), 100 | 0x100); // 356
    assert_eq!(ch0.vstop(), 120 | 0x100); // 376
    assert_eq!(ch0.hstart(), (0x32 << 1) | 1); // 101
    assert!(ch0.is_attached());

    sprites.set_data(0, 0xAAAA, 0x5555);
    assert!(sprites.channels[0].is_armed);

    sprites.reset();
    assert_eq!(sprites.channels[0].vstart(), 0);
    assert_eq!(sprites.channels[0].vstop(), 0);
    assert_eq!(sprites.channels[0].hstart(), 0);
    assert!(!sprites.channels[0].is_armed);
}

#[test]
fn test_sprite_vertical_comparator_and_scanlines() {
    let mut sprites = Sprites::new();
    // VSTART = 100, VSTOP = 105, HSTART = 50
    sprites.set_pos(0, (100 << 8) | 50);
    sprites.set_ctl(0, 105 << 8);

    // Line 99: not active
    sprites.step_cck(beam(99));
    assert!(!sprites.channels[0].is_active_line);

    // Lines 100..104: active
    for line in 100..105 {
        sprites.step_cck(beam(line));
        assert!(
            sprites.channels[0].is_active_line,
            "Line {} should be active",
            line
        );
    }

    // Line 105: stop position reached -> not active
    sprites.step_cck(beam(105));
    assert!(!sprites.channels[0].is_active_line);
}

#[test]
fn test_sprite_horizontal_arming_and_shift_serialization() {
    let mut sprites = Sprites::new();
    // VSTART = 50, VSTOP = 60, HSTART = 100 (low=50, high=0 -> 100)
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);

    // Data: bit 15 = 1 in A, bit 15 = 0 in B -> pixel color 1
    // bit 14 = 1 in A, bit 14 = 1 in B -> pixel color 3
    sprites.set_data(0, 0xC000, 0x4000);
    assert!(sprites.channels[0].is_armed);

    // Scanline 50
    sprites.step_cck(beam(50));

    let mut clxdat = 0u16;

    // Before HSTART (pixel 99): no sprite pixel
    let p_before = sprites.evaluate_pixel(99, &mut clxdat);
    assert_eq!(p_before, None);

    // At HSTART (pixel 100): pixel 0 of sprite (bit 15: bit_a=1, bit_b=0 -> 1)
    // For Sprite 0 (pair 0), color index is 16 + 0*4 + 1 = 17
    let p0 = sprites.evaluate_pixel(100, &mut clxdat);
    assert_eq!(
        p0,
        Some(SpritePixel {
            color_index: 17,
            sprite_pair: 0,
        })
    );

    // Pixel 101: bit 14 (bit_a=1, bit_b=1 -> 3) -> color index 16 + 3 = 19
    let p1 = sprites.evaluate_pixel(101, &mut clxdat);
    assert_eq!(
        p1,
        Some(SpritePixel {
            color_index: 19,
            sprite_pair: 0,
        })
    );

    // Next 14 pixels: data is 0, so pixel value is 0 (transparent)
    for px in 102..=115 {
        let p = sprites.evaluate_pixel(px, &mut clxdat);
        assert_eq!(p, None, "Pixel at {} should be transparent", px);
    }

    // Pixel 116: 16 pixels have completed, shift register empty
    let p_after = sprites.evaluate_pixel(116, &mut clxdat);
    assert_eq!(p_after, None);
}

#[test]
fn test_sprite_disarming_on_ctl_write() {
    let mut sprites = Sprites::new();
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);
    sprites.set_data(0, 0xFFFF, 0xFFFF);
    assert!(sprites.channels[0].is_armed);

    // Writing CTL disarms the horizontal comparator
    sprites.set_ctl(0, 60 << 8);
    assert!(!sprites.channels[0].is_armed);

    // Re-writing DATA re-arms it
    sprites.set_data(0, 0xFFFF, 0xFFFF);
    assert!(sprites.channels[0].is_armed);
}

#[test]
fn test_sprite_attached_mode_15_colors() {
    let mut sprites = Sprites::new();
    // Pair 0: Sprite 0 (even) and Sprite 1 (odd)
    // Both positioned at line 50, HSTART = 100
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);

    // Sprite 1 attached (bit 7 ATT = 1)
    sprites.set_pos(1, (50 << 8) | 50);
    sprites.set_ctl(1, (60 << 8) | 0x0080);
    assert!(sprites.channels[1].is_attached());

    // Sprite 0: bit 15 has data_a=1, data_b=0 (value = 1)
    sprites.set_data(0, 0x8000, 0x0000);
    // Sprite 1: bit 15 has data_a=1, data_b=1 (value = 3)
    sprites.set_data(1, 0x8000, 0x8000);

    sprites.step_cck(beam(50));

    let mut clxdat = 0u16;
    // At HSTART = 100:
    // Even sprite value = 1 (bits 0..1)
    // Odd sprite value = 3 (bits 2..3) -> 3 << 2 = 12
    // Combined = 12 | 1 = 13
    // Color index = 16 + 13 = 29
    let pixel = sprites.evaluate_pixel(100, &mut clxdat);
    assert_eq!(
        pixel,
        Some(SpritePixel {
            color_index: 29,
            sprite_pair: 0,
        })
    );
}

#[test]
fn test_sprite_to_sprite_collision_clxdat() {
    let mut sprites = Sprites::new();

    // Sprite 0 (Pair 0) at HSTART = 100, line 50
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);
    sprites.set_data(0, 0x8000, 0x8000); // 1 active pixel at px 100

    // Sprite 2 (Pair 1) at HSTART = 100, line 50
    sprites.set_pos(2, (50 << 8) | 50);
    sprites.set_ctl(2, 60 << 8);
    sprites.set_data(2, 0x8000, 0x8000); // 1 active pixel at px 100

    // Sprite 4 (Pair 2) at HSTART = 100, line 50
    sprites.set_pos(4, (50 << 8) | 50);
    sprites.set_ctl(4, 60 << 8);
    sprites.set_data(4, 0x8000, 0x8000); // 1 active pixel at px 100

    sprites.step_cck(beam(50));

    let mut clxdat = 0u16;
    let _ = sprites.evaluate_pixel(100, &mut clxdat);

    // Collisions:
    // Pair 0 with Pair 1 -> bit 9 (1 << 9 = 0x0200)
    // Pair 0 with Pair 2 -> bit 10 (1 << 10 = 0x0400)
    // Pair 1 with Pair 2 -> bit 12 (1 << 12 = 0x1000)
    assert_ne!(clxdat & (1 << 9), 0, "Pair 0 to Pair 1 collision bit 9");
    assert_ne!(clxdat & (1 << 10), 0, "Pair 0 to Pair 2 collision bit 10");
    assert_ne!(clxdat & (1 << 12), 0, "Pair 1 to Pair 2 collision bit 12");
}

#[test]
fn test_sprite_priority_arbitration() {
    let mut sprites = Sprites::new();

    // Pair 0 (Sprite 0): color 1 -> color_index = 17
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);
    sprites.set_data(0, 0x8000, 0x0000);

    // Pair 1 (Sprite 2): color 3 -> color_index = 16 + 4 + 3 = 23
    sprites.set_pos(2, (50 << 8) | 50);
    sprites.set_ctl(2, 60 << 8);
    sprites.set_data(2, 0x8000, 0x8000);

    sprites.step_cck(beam(50));

    let mut clxdat = 0u16;
    let p = sprites.evaluate_pixel(100, &mut clxdat);

    // Sprite 0 must win priority over Sprite 2
    assert_eq!(
        p,
        Some(SpritePixel {
            color_index: 17,
            sprite_pair: 0,
        })
    );
}

#[test]
fn test_sprite_multiplexing() {
    let mut sprites = Sprites::new();

    // First use: lines 50..60, HSTART = 100
    sprites.set_pos(0, (50 << 8) | 50);
    sprites.set_ctl(0, 60 << 8);
    sprites.set_data(0, 0x8000, 0x0000);

    sprites.step_cck(beam(50));
    let mut clxdat = 0u16;
    let p1 = sprites.evaluate_pixel(100, &mut clxdat);
    assert_eq!(
        p1,
        Some(SpritePixel {
            color_index: 17,
            sprite_pair: 0
        })
    );

    // Past line 60: inactive
    sprites.step_cck(beam(70));
    let p_inactive = sprites.evaluate_pixel(100, &mut clxdat);
    assert_eq!(p_inactive, None);

    // Reuse channel 0: lines 150..160, HSTART = 120
    sprites.set_pos(0, (150 << 8) | 60);
    sprites.set_ctl(0, 160 << 8);
    sprites.set_data(0, 0x0000, 0x8000); // color 2 -> 16 + 2 = 18

    sprites.step_cck(beam(150));
    let p2 = sprites.evaluate_pixel(120, &mut clxdat);
    assert_eq!(
        p2,
        Some(SpritePixel {
            color_index: 18,
            sprite_pair: 0
        })
    );
}

#[test]
fn test_sprite_dma_enable_transition_and_idempotence() {
    let mut sprites = Sprites::new();

    // Arm sprite 0 manually
    sprites.set_data(0, 0x1234, 0x5678);
    assert!(sprites.channels[0].is_armed);

    // Redundant disable while already disabled must NOT disarm manually loaded sprite
    sprites.set_dma_enabled(false);
    assert!(
        sprites.channels[0].is_armed,
        "Redundant disable must not disarm"
    );

    // Enable DMA
    sprites.set_dma_enabled(true);
    assert!(sprites.dma_enabled);
    assert!(sprites.channels[0].is_armed);

    // Falling edge transition from true to false MUST disarm channels
    sprites.set_dma_enabled(false);
    assert!(!sprites.dma_enabled);
    assert!(
        !sprites.channels[0].is_armed,
        "Falling edge must disarm channels"
    );
}
