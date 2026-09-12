use audio::Audio;

#[test]
fn test_audio_channel_configuration() {
    let mut audio = Audio::new();
    audio.set_loc(0, 0x00020000);
    audio.set_len(0, 128);
    audio.set_per(0, 350);
    audio.set_vol(0, 64);
    audio.set_dat(0, 0x1234);

    assert_eq!(audio.channels[0].lc, 0x00020000);
    assert_eq!(audio.channels[0].len, 128);
    assert_eq!(audio.channels[0].per, 350);
    assert_eq!(audio.channels[0].vol, 64);
    assert!(audio.channels[0].active);

    audio.reset();
    assert!(!audio.channels[0].active);
    assert_eq!(audio.channels[0].vol, 0);
}
