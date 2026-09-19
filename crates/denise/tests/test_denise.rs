#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use denise::{Denise, DeniseModel};

#[test]
fn test_denise_reset_and_palette() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    assert_eq!(denise.color[0], 0);

    // Write white: 0x0FFF
    denise.color[0] = 0x0FFF;
    assert_eq!(denise.color[0], 0x0FFF);

    // CLXDAT clear-on-read
    denise.clxdat = 0x1234;
    assert_eq!(denise.read_clxdat(), 0x1234);
    assert_eq!(denise.read_clxdat(), 0);

    denise.reset();
    assert_eq!(denise.color[0], 0);
}

#[test]
fn test_denise_sprite_arming_on_data_write() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);

    // Initial state: not armed
    assert!(!denise.sprites.channels[0].is_armed);

    // Writing SPR0DATA ($144) arms the channel
    denise.write_register(0x144, 0x1234);
    denise.step_cck(config::BeamPosition::default());
    assert!(denise.sprites.channels[0].is_armed);
    assert_eq!(denise.sprites.channels[0].data_a, 0x1234);

    // Writing SPR0CTL ($142) disarms the channel
    denise.write_register(0x142, 0x5678);
    denise.step_cck(config::BeamPosition::default());
    assert!(!denise.sprites.channels[0].is_armed);
}

#[test]
fn test_denise_batch_palette_mutations_capacity() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);

    // Write all 32 color registers in batch
    for i in 0..32u16 {
        denise.write_register(0x180 + i * 2, 0x0A00 + i);
    }

    // Step CCK to mature and commit mutations
    denise.step_cck(config::BeamPosition::default());

    // Verify all 32 entries committed
    for i in 0..32 {
        assert_eq!(denise.color[i], 0x0A00 + i as u16);
    }
}

#[test]
fn test_denise_write_bpldat_and_dma_reload() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.set_bplcon0(0x1200); // 1 bitplane
    denise.ddfstrt = 0x38;
    denise.ddfstop = 0xD0;

    // Manual write when DMA is disabled reloads immediately on plane 0
    denise.write_bpldat(0, 0x1234);
    assert_eq!(denise.shifters[0], 0x1234);

    // When DMA is enabled, writes go into bpldat and reload at block boundary
    denise.frame_builder.dma_enabled = true;
    denise.write_bpldat(0, 0x5678);
    // Not reloaded yet
    assert_ne!(denise.shifters[0], 0x5678);

    // Advance to start of next block 0x40 (hpos = 0x40, phase = 0)
    let beam_40 = config::BeamPosition::new(0x40, 50, false);
    denise.step_cck(beam_40);
    // Reloads 0x5678 and shifts out 2 low-res pixels (2 bits) on this CCK
    assert_eq!(denise.shifters[0], 0x5678 << 2);
}

#[test]
fn test_denise_vblank_and_hblank_analog_black() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.color[0] = 0x0FFF; // Backdrop is white

    // VBlank line 10: pixel buffer must receive analog blanking (pure black 0xFF00_0000)
    let beam_vblank = config::BeamPosition::new(100, 10, false);
    denise.step_cck(beam_vblank);
    let px = denise.frame_builder.get_pixel(100 * 4, 10);
    assert_eq!(px, 0xFF00_0000);

    // Active line 50, but during HBlank (CCK 20): pure black
    let beam_hblank = config::BeamPosition::new(20, 50, false);
    denise.step_cck(beam_hblank);
    let px_hb = denise.frame_builder.get_pixel(20 * 4, 50);
    assert_eq!(px_hb, 0xFF00_0000);
}

#[test]
fn test_denise_short_line_cck227_edge_coverage() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.color[0] = 0x00F0; // Backdrop is green (0x00F0 -> ARGB 0xFF00F000)

    // On an active line (vpos = 50), stepping at CCK 226 (last CCK of short line)
    // must write both CCK 226 and CCK 227 so that the 912-pixel viewport row is fully filled
    let beam_226 = config::BeamPosition::new(226, 50, false);
    denise.step_cck(beam_226);

    let px_226 = denise.frame_builder.get_pixel(226 * 4, 50);
    let px_227 = denise.frame_builder.get_pixel(227 * 4, 50);
    assert_eq!(px_226, 0xFF00_F000);
    assert_eq!(px_227, 0xFF00_F000);
}

#[test]
fn test_denise_color_write_immediate_commit_timing() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);

    // Initial backdrop color is black
    assert_eq!(denise.color[0], 0x0000);

    // Writing COLOR00 via write_register must commit immediately on the active cycle (0 delay)
    let res = denise.write_register(0x180, 0x0F0F);
    assert_eq!(res, Some((0x180, 0x0F0F)));
    assert_eq!(denise.color[0], 0x0F0F);

    // Stepping CCK at an active beam coordinate draws using the updated color immediately
    let beam = config::BeamPosition::new(50, 50, false);
    denise.step_cck(beam);
    let px = denise.frame_builder.get_pixel(50 * 4, 50);
    assert_eq!(px, 0xFFF0_00F0);
}

#[test]
fn test_denise_hflop_comparator() {
    let mut denise = Denise::new(config::DeniseModel::Ocs8362);
    denise.frame_builder.set_dma_enabled(true);
    // Standard PAL Display Window: HSTRT = $81 (129), HSTOP = $1C1 (449)
    denise.write_register(0x08E, 0x2C81); // DIWSTRT: V=44, H=129
    denise.write_register(0x090, 0xF4C1); // DIWSTOP: V=500, H=449
    assert!(!denise.hflop);

    // Before HSTRT: hpos = 60 (c0 = 122, c1 = 123) -> hflop is false
    let beam_before = config::BeamPosition::new(60, 50, false);
    denise.step_cck(beam_before);
    assert!(!denise.hflop);

    // At HSTRT: hpos = 63 (c0 = 128, c1 = 129 == hstrt) -> hflop latches true
    let beam_start = config::BeamPosition::new(63, 50, false);
    denise.step_cck(beam_start);
    assert!(denise.hflop);

    // Inside DIW: hpos = 100 -> hflop remains true
    let beam_mid = config::BeamPosition::new(100, 50, false);
    denise.step_cck(beam_mid);
    assert!(denise.hflop);

    // At HSTOP: hpos = 223 (c1 = 449 == hstop) -> hflop latches false after sampling
    let beam_stop = config::BeamPosition::new(223, 50, false);
    denise.step_cck(beam_stop);
    assert!(!denise.hflop);
}

#[test]
fn test_denise_bplcon0_modes() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.set_bplcon0(0x9200); // HIRES | 1 plane | COLOR
    assert!(denise.is_hires());
    assert_eq!(denise.bitplane_count(), 1);
}
