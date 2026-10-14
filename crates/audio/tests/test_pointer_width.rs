use audio::Audio;

#[test]
fn sample_location_discards_bits_outside_ocs_dma_address() {
    let mut audio = Audio::new();

    audio.set_loc(0, 0x001D_2345);

    assert_eq!(audio.channels[0].lc, 0x0005_2344);
    assert_eq!(audio.channels[0].current_pt, 0x0005_2344);
}
