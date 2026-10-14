use config::custom_reg::agnus;
use paula::Paula;

#[test]
fn direct_audio_location_writes_discard_unused_address_bits() {
    let mut paula = Paula::new();

    for (channel, high, low) in [
        (0, agnus::AUD0LCH, agnus::AUD0LCL),
        (1, agnus::AUD1LCH, agnus::AUD1LCL),
        (2, agnus::AUD2LCH, agnus::AUD2LCL),
        (3, agnus::AUD3LCH, agnus::AUD3LCL),
    ] {
        paula.commit_register_write(high, 0x001D);
        paula.commit_register_write(low, 0x2345);

        assert_eq!(paula.audio.channels[channel].lc, 0x0005_2344);
        assert_eq!(paula.audio.channels[channel].current_pt, 0x0005_2344);
    }
}
