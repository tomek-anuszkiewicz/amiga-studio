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
