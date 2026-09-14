use config::BeamPosition;
use frame_builder::{
    is_in_display_window, rgb444_to_argb32, FrameBuilder, FRAME_BUFFER_PIXELS, MAX_FRAME_HEIGHT,
    MAX_FRAME_WIDTH,
};

#[test]
fn test_rgb444_to_argb32_color_conversion() {
    // Primary and baseline colors
    assert_eq!(rgb444_to_argb32(0x0000), 0xFF000000); // Black
    assert_eq!(rgb444_to_argb32(0x0FFF), 0xFFFFFFFF); // White
    assert_eq!(rgb444_to_argb32(0x0F00), 0xFFFF0000); // Red
    assert_eq!(rgb444_to_argb32(0x00F0), 0xFF00FF00); // Green
    assert_eq!(rgb444_to_argb32(0x000F), 0xFF0000FF); // Blue

    // Nibble replication (0xR -> 0xRR, 0xG -> 0xGG, 0xB -> 0xBB)
    // 0x0A5C: R=0xA -> 0xAA, G=0x5 -> 0x55, B=0xC -> 0xCC
    assert_eq!(rgb444_to_argb32(0x0A5C), 0xFFAA55CC);
    // 0x0123: R=0x1 -> 0x11, G=0x2 -> 0x22, B=0x3 -> 0x33
    assert_eq!(rgb444_to_argb32(0x0123), 0xFF112233);

    // Ensure highest nibble (bits 15-12) is ignored
    assert_eq!(rgb444_to_argb32(0xFA5C), 0xFFAA55CC);
}

#[test]
fn test_is_in_display_window_standard_and_msb_extended() {
    // Standard PAL window:
    // DIWSTRT = 0x2C81 (vstart = 0x2C = 44, hstart = 0x81 = 129)
    // DIWSTOP = 0x2DC1 (vstop = 0x2D | 0x100 = 0x12D = 301 since bit 15 is 0, hstop = 0xC1 | 0x100 = 0x1C1 = 449)
    let diwstrt = 0x2C81;
    let diwstop = 0x2DC1;

    // Inside window
    assert!(is_in_display_window(129, 44, diwstrt, diwstop));
    assert!(is_in_display_window(200, 100, diwstrt, diwstop));
    assert!(is_in_display_window(448, 300, diwstrt, diwstop));

    // Outside horizontal bounds
    assert!(!is_in_display_window(128, 44, diwstrt, diwstop));
    assert!(!is_in_display_window(449, 44, diwstrt, diwstop));

    // Outside vertical bounds
    assert!(!is_in_display_window(129, 43, diwstrt, diwstop));
    assert!(!is_in_display_window(129, 301, diwstrt, diwstop));

    // Bit 15 of DIWSTOP set: bit 15 is 1, so (diwstop & 0x8000) != 0 suppresses the 0x100 extension.
    // DIWSTOP = 0x90C1 -> (0x90C1 >> 8) & 0xFF = 0x90 = 144, vstop = 144 (without 0x100 added)
    let diwstop_msb = 0x90C1;
    assert!(is_in_display_window(129, 44, diwstrt, diwstop_msb));
    assert!(is_in_display_window(129, 143, diwstrt, diwstop_msb));
    assert!(!is_in_display_window(129, 144, diwstrt, diwstop_msb));
}

#[test]
fn test_frame_builder_pixel_manipulation_and_bounds() {
    let mut fb = FrameBuilder::new();
    assert_eq!(fb.frame_buffer().len(), FRAME_BUFFER_PIXELS);
    assert_eq!(fb.width, MAX_FRAME_WIDTH as u32);
    assert_eq!(fb.height, MAX_FRAME_HEIGHT as u32);

    // Initial pixels are black
    assert_eq!(fb.get_pixel(0, 0), 0xFF000000);
    assert_eq!(
        fb.get_pixel(MAX_FRAME_WIDTH - 1, MAX_FRAME_HEIGHT - 1),
        0xFF000000
    );

    // Set and get valid pixels
    fb.set_pixel(100, 50, 0xFF123456);
    assert_eq!(fb.get_pixel(100, 50), 0xFF123456);

    // Out of bounds get returns 0xFF000000
    assert_eq!(fb.get_pixel(MAX_FRAME_WIDTH, 0), 0xFF000000);
    assert_eq!(fb.get_pixel(0, MAX_FRAME_HEIGHT), 0xFF000000);

    // Out of bounds set is safely ignored
    fb.set_pixel(MAX_FRAME_WIDTH, 0, 0xFFFFFFFF);
    fb.set_pixel(0, MAX_FRAME_HEIGHT, 0xFFFFFFFF);

    // Frame buffer mut
    fb.frame_buffer_mut()[0] = 0xFFABCDEF;
    assert_eq!(fb.get_pixel(0, 0), 0xFFABCDEF);
}

#[test]
fn test_frame_builder_set_cck_pixels() {
    let mut fb = FrameBuilder::new();
    let hpos = 10;
    let vpos = 20;
    let color = 0xFF55AAFF;

    fb.set_cck_pixels(hpos, vpos, color);

    for offset in 0..4 {
        assert_eq!(
            fb.get_pixel(hpos as usize * 4 + offset, vpos as usize),
            color
        );
    }

    // Out of bounds CCK pixels are safely ignored
    let oob_hpos = (MAX_FRAME_WIDTH / 4) as u16;
    fb.set_cck_pixels(oob_hpos, vpos, 0xFFFFFFFF);
    let oob_vpos = MAX_FRAME_HEIGHT as u16;
    fb.set_cck_pixels(hpos, oob_vpos, 0xFFFFFFFF);
}

#[test]
fn test_frame_builder_step_cck_and_frame_lifecycle() {
    let mut fb = FrameBuilder::new();
    assert!(!fb.frame_ready);

    // Advance to arbitrary beam position
    fb.step_cck(BeamPosition::new(100, 200, false));
    assert_eq!(fb.hpos, 100);
    assert_eq!(fb.vpos, 200);
    assert!(!fb.frame_ready);

    // Beam wrapping to (0, 0) marks frame ready
    fb.step_cck(BeamPosition::new(0, 0, false));
    assert_eq!(fb.hpos, 0);
    assert_eq!(fb.vpos, 0);
    assert!(fb.frame_ready);

    // begin_frame clears frame_ready and positions
    fb.begin_frame();
    assert!(!fb.frame_ready);
    assert_eq!(fb.hpos, 0);
    assert_eq!(fb.vpos, 0);

    // end_frame manually signals frame ready
    fb.end_frame();
    assert!(fb.frame_ready);
}

#[test]
fn test_frame_builder_dma_enabled_and_reset() {
    let mut fb = FrameBuilder::new();
    assert!(!fb.dma_enabled);

    fb.set_dma_enabled(true);
    assert!(fb.dma_enabled);

    fb.set_pixel(10, 10, 0xFFFFFFFF);
    fb.end_frame();
    fb.hpos = 50;
    fb.vpos = 100;

    fb.reset();
    assert_eq!(fb.vpos, 0);
    assert_eq!(fb.hpos, 0);
    assert!(!fb.dma_enabled);
    assert!(!fb.frame_ready);
    assert_eq!(fb.get_pixel(10, 10), 0xFF000000);
}

#[test]
fn test_extract_vamiga_raw_viewport() {
    let mut fb = FrameBuilder::new();
    // Fill a specific pixel inside the vAmiga viewport [196, 912) x [26, 311)
    // At x = 196, y = 26 (the very first pixel of the viewport):
    fb.set_pixel(196, 26, 0xFFAABBCC); // R=AA, G=BB, B=CC

    let mut out = [0u8; 612_180];
    fb.extract_vamiga_raw_viewport(&mut out);

    // Byte 0: R, Byte 1: G, Byte 2: B
    assert_eq!(out[0], 0xAA);
    assert_eq!(out[1], 0xBB);
    assert_eq!(out[2], 0xCC);

    // Second pixel is black
    assert_eq!(out[3], 0x00);
    assert_eq!(out[4], 0x00);
    assert_eq!(out[5], 0x00);
}

#[test]
fn test_frame_builder_default_and_equality() {
    let fb1 = FrameBuilder::default();
    let fb2 = FrameBuilder::new();
    assert_eq!(fb1, fb2);
}
