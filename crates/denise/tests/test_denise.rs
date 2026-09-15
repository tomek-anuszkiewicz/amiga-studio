use denise::{Denise, DeniseModel};

#[test]
fn test_denise_reset_and_palette() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    assert_eq!(denise.read_color(0), 0);

    // Write white: 0x0FFF
    denise.write_color(0, 0x0FFF);
    assert_eq!(denise.read_color(0), 0x0FFF);

    // CLXDAT clear-on-read
    denise.clxdat = 0x1234;
    assert_eq!(denise.read_clxdat(), 0x1234);
    assert_eq!(denise.read_clxdat(), 0);

    denise.reset();
    assert_eq!(denise.read_color(0), 0);
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

    // Advance to end of block 0x38 (hpos = 0x3F, phase = 7)
    let beam_3f = config::BeamPosition::new(0x3F, 50, false);
    denise.step_cck(beam_3f);
    assert_eq!(denise.bpldat_pipe[0], 0x5678);

    // Advance to start of display (hpos = 0x40, phase = 8)
    let beam_40 = config::BeamPosition::new(0x40, 50, false);
    denise.step_cck(beam_40);
    assert_eq!(denise.shifters[0], 0x5678);
}

#[test]
fn test_denise_vblank_and_hblank_analog_black() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.write_color(0, 0x0FFF); // Backdrop is white

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
