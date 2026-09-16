use config::DeniseModel;
use denise::{decode_dual_playfield, decode_ehb, decode_ham6, frame_builder, Denise};

#[test]
fn test_rgb444_to_argb32() {
    assert_eq!(frame_builder::rgb444_to_argb32(0x000), 0xFF00_0000);
    assert_eq!(frame_builder::rgb444_to_argb32(0xFFF), 0xFFF0_F0F0);
    assert_eq!(frame_builder::rgb444_to_argb32(0xF00), 0xFFF0_0000);
    assert_eq!(frame_builder::rgb444_to_argb32(0x0F0), 0xFF00_F000);
    assert_eq!(frame_builder::rgb444_to_argb32(0x00F), 0xFF00_00F0);
    assert_eq!(frame_builder::rgb444_to_argb32(0x123), 0xFF10_2030);
    assert_eq!(frame_builder::rgb444_to_argb32(0xABC), 0xFFA0_B0C0);
}

#[test]
fn test_display_window_clipping() {
    // Standard PAL window: VSTART = 0x2C (44), VSTOP = 0x12C (300 -> 0x2C in low byte)
    // HSTART = 0x81 (129), HSTOP = 0x1C1 (449 -> 0xC1 in low byte)
    let diwstrt = 0x2C81;
    let diwstop = 0x2CC1; // 0x2C in high byte (< 0x2C -> 256 + 0x2C = 300)

    // Inside window
    assert!(frame_builder::is_in_display_window(
        130, 45, diwstrt, diwstop
    ));
    assert!(frame_builder::is_in_display_window(
        200, 150, diwstrt, diwstop
    ));
    assert!(frame_builder::is_in_display_window(
        448, 299, diwstrt, diwstop
    ));

    // Outside window (border / blanking)
    assert!(!frame_builder::is_in_display_window(
        128, 45, diwstrt, diwstop
    )); // Left of HSTART
    assert!(!frame_builder::is_in_display_window(
        450, 150, diwstrt, diwstop
    )); // Right of HSTOP
    assert!(!frame_builder::is_in_display_window(
        200, 40, diwstrt, diwstop
    )); // Above VSTART
    assert!(!frame_builder::is_in_display_window(
        200, 301, diwstrt, diwstop
    )); // Below VSTOP
}

#[test]
fn test_bitplane_serialization_and_palette_lookup() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    // Enable 4 bitplanes in Low-Res (BPU = 4 -> 0x4200)
    denise.set_bplcon0(0x4200);
    assert_eq!(denise.bitplane_count(), 4);
    assert!(!denise.is_hires());

    // Setup color palette
    denise.set_color(0, 0x000); // Black
    denise.set_color(1, 0xF00); // Red
    denise.set_color(2, 0x0F0); // Green
    denise.set_color(3, 0x00F); // Blue
    denise.set_color(15, 0xFFF); // White

    // Bitplane word test:
    // Pixel 0: plane 0 = 1, planes 1..3 = 0 -> color 1 (Red)
    // Pixel 1: plane 1 = 1, planes 0,2,3 = 0 -> color 2 (Green)
    // Pixel 2: plane 0 = 1, plane 1 = 1 -> color 3 (Blue)
    // Pixel 3: all planes 0..3 = 1 -> color 15 (White)
    let p0 = 0b1011_0000_0000_0000u16;
    let p1 = 0b0111_0000_0000_0000u16;
    let p2 = 0b0001_0000_0000_0000u16;
    let p3 = 0b0001_0000_0000_0000u16;

    denise.load_bitplane_data([p0, p1, p2, p3, 0, 0]);

    // Pixel 0
    let pix0 = denise.shift_pixel();
    assert_eq!(pix0, 1);
    assert_eq!(denise.decode_pixel(pix0), 0xF00);

    // Pixel 1
    let pix1 = denise.shift_pixel();
    assert_eq!(pix1, 2);
    assert_eq!(denise.decode_pixel(pix1), 0x0F0);

    // Pixel 2
    let pix2 = denise.shift_pixel();
    assert_eq!(pix2, 3);
    assert_eq!(denise.decode_pixel(pix2), 0x00F);

    // Pixel 3
    let pix3 = denise.shift_pixel();
    assert_eq!(pix3, 15);
    assert_eq!(denise.decode_pixel(pix3), 0xFFF);
}

#[test]
fn test_extra_half_brite_mode() {
    let mut palette = [0u16; 32];
    palette[1] = 0xF64; // R=15, G=6, B=4
    palette[10] = 0x8AE; // R=8, G=10, B=14

    // Plane 6 is 0 (bits 0..4 select color 1)
    let normal_pixel = 0x01; // Bit 5 is 0
    assert_eq!(decode_ehb(normal_pixel, &palette), 0xF64);

    // Plane 6 is 1 (bit 5 is 1, lower 5 bits select color 1)
    // Halved RGB components: R=15>>1 = 7, G=6>>1 = 3, B=4>>1 = 2 -> 0x732
    let ehb_pixel = 0x21; // 0x20 | 0x01
    assert_eq!(decode_ehb(ehb_pixel, &palette), 0x732);

    // Test second color halved: R=8>>1 = 4, G=10>>1 = 5, B=14>>1 = 7 -> 0x457
    let ehb_pixel10 = 0x2A; // 0x20 | 10
    assert_eq!(decode_ehb(ehb_pixel10, &palette), 0x457);
}

#[test]
fn test_ham6_mode() {
    let mut palette = [0u16; 32];
    palette[0] = 0x000;
    palette[1] = 0x888; // Gray
    palette[5] = 0x246;

    let mut held_rgb = 0x000u16;

    // 1. Control 00: Palette lookup (select color 5) -> 0x246
    let p_lookup = 0b00_0101; // Control 00, data 5
    assert_eq!(decode_ham6(p_lookup, &palette, &mut held_rgb), 0x246);
    assert_eq!(held_rgb, 0x246);

    // 2. Control 01: Modify Blue (keep R=2, G=4, replace B with 15) -> 0x24F
    let p_mod_b = 0b01_1111; // Control 01, data 15
    assert_eq!(decode_ham6(p_mod_b, &palette, &mut held_rgb), 0x24F);
    assert_eq!(held_rgb, 0x24F);

    // 3. Control 10: Modify Red (replace R with 14, keep G=4, B=15) -> 0xE4F
    let p_mod_r = 0b10_1110; // Control 10, data 14
    assert_eq!(decode_ham6(p_mod_r, &palette, &mut held_rgb), 0xE4F);
    assert_eq!(held_rgb, 0xE4F);

    // 4. Control 11: Modify Green (keep R=14, replace G with 1, keep B=15) -> 0xE1F
    let p_mod_g = 0b11_0001; // Control 11, data 1
    assert_eq!(decode_ham6(p_mod_g, &palette, &mut held_rgb), 0xE1F);
    assert_eq!(held_rgb, 0xE1F);
}

#[test]
fn test_dual_playfield_layering_and_priority() {
    let mut palette = [0u16; 32];
    palette[0] = 0x111; // Backdrop
    palette[1] = 0xF00; // PF1 color 1 (Red)
    palette[2] = 0x0F0; // PF1 color 2 (Green)
    palette[8 + 1] = 0x00F; // PF2 color 1 (Blue)
    palette[8 + 2] = 0xFF0; // PF2 color 2 (Yellow)

    // Pixel with PF1 = 1 (bit 0 = 1), PF2 = 0 -> Red
    let p_pf1_only = 0b000001;
    assert_eq!(decode_dual_playfield(p_pf1_only, &palette, false), 0xF00);

    // Pixel with PF1 = 0, PF2 = 1 (bit 1 = 1) -> Blue
    let p_pf2_only = 0b000010;
    assert_eq!(decode_dual_playfield(p_pf2_only, &palette, false), 0x00F);

    // Overlapping pixel: PF1 = 1 (Red), PF2 = 2 (Yellow)
    // Bits: plane 0 = 1 (PF1=1), plane 1 = 0 (PF2 low bit 0), plane 3 = 1 (PF2 high bit 1)
    let p_both = 0b001001;

    // Priority 0: PF1 in front -> Red (0xF00)
    assert_eq!(decode_dual_playfield(p_both, &palette, false), 0xF00);

    // Priority 1: PF2 in front -> Yellow (0xFF0)
    assert_eq!(decode_dual_playfield(p_both, &palette, true), 0xFF0);

    // Both transparent (0) -> Backdrop (0x111)
    assert_eq!(decode_dual_playfield(0, &palette, false), 0x111);
}

#[test]
fn test_render_scanline_with_fine_scrolling() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.set_bplcon0(0x2200); // 2 bitplanes
    denise.set_color(0, 0x000); // Black backdrop
    denise.set_color(1, 0xFFF); // White

    // BPLCON1 = 4 (4-pixel fine scroll delay)
    denise.set_bplcon1(0x0004);

    let block = [0xFFFF, 0x0000, 0, 0, 0, 0]; // 16 white pixels
    denise.render_scanline(100, &[block]);

    // Pixels 0..4 should be black backdrop (fine scroll delay padding)
    for x in 0..4 {
        assert_eq!(denise.frame_builder.get_pixel(x, 100), 0xFF00_0000);
    }
    // Pixels 4..20 should be white (0xFFF0F0F0)
    for x in 4..20 {
        assert_eq!(denise.frame_builder.get_pixel(x, 100), 0xFFF0_F0F0);
    }
}

#[test]
fn test_pipeline_pixels_latency_and_backdrop_immediacy() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.set_bplcon0(0x1200); // 1 bitplane, low-res
    denise.set_color(0, 0xF00); // Red backdrop
    denise.set_color(1, 0x0F0); // Green foreground
    denise.frame_builder.dma_enabled = true;
    denise.set_diw(0x2C81, 0x2CC1); // Standard PAL display window

    // When bpl_armed is false: all 4 pixels of CCK receive backdrop immediately
    let beam = config::BeamPosition::new(50, 100, false);
    denise.step_cck(beam);
    let red_argb = frame_builder::rgb444_to_argb32(0xF00);
    let green_argb = frame_builder::rgb444_to_argb32(0x0F0);

    for px in 0..4 {
        assert_eq!(
            denise.frame_builder.get_pixel(50 * 4 + px, 100),
            red_argb,
            "Pixel at offset {} should be immediate red backdrop when bpl_armed is false",
            px
        );
    }

    // Arm bitplanes by writing BPL1DAT: $C000 (two 1s followed by zeroes)
    denise.write_bpldat(0, 0xC000);
    assert!(denise.bpl_armed);

    // Step across HSTART (hstrt = 129, triggered at CCK 63: c1 = 63*2+3 = 129)
    for h in 62..64 {
        let b = config::BeamPosition::new(h, 100, false);
        denise.step_cck(b);
    }
    assert!(denise.hflop);

    // In Low-Res, CCK 64 (inside DIW, and 64 % 8 == 0 for shifter reload):
    // p0 = 1 (Green), p1 = 1 (Green).
    // Due to 2-hires-pixel latency:
    // px0, px1 receive previous pipeline (red backdrop)
    // px2, px3 receive p0 (Green foreground)
    // p1 is staged in pipeline_pixels for the next CCK
    let beam2 = config::BeamPosition::new(64, 100, false);
    denise.step_cck(beam2);

    assert_eq!(
        denise.frame_builder.get_pixel(64 * 4 + 0, 100),
        red_argb,
        "px0 should be un-delayed backdrop before pipeline flush"
    );
    assert_eq!(
        denise.frame_builder.get_pixel(64 * 4 + 1, 100),
        red_argb,
        "px1 should be un-delayed backdrop before pipeline flush"
    );
    assert_eq!(
        denise.frame_builder.get_pixel(64 * 4 + 2, 100),
        green_argb,
        "px2 should be delayed foreground pixel"
    );
    assert_eq!(
        denise.frame_builder.get_pixel(64 * 4 + 3, 100),
        green_argb,
        "px3 should be delayed foreground pixel"
    );

    // Next CCK 65: receives staged p1 in px0, px1
    let beam3 = config::BeamPosition::new(65, 100, false);
    denise.step_cck(beam3);

    assert_eq!(
        denise.frame_builder.get_pixel(65 * 4 + 0, 100),
        green_argb,
        "px0 of subsequent CCK should receive staged trailing foreground pixel"
    );
    assert_eq!(
        denise.frame_builder.get_pixel(65 * 4 + 1, 100),
        green_argb,
        "px1 of subsequent CCK should receive staged trailing foreground pixel"
    );
}
