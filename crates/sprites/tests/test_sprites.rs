use sprites::Sprites;

#[test]
fn test_sprite_decoding_and_reset() {
    let mut sprites = Sprites::new();
    // Set Sprite 0: VSTART = 100 ($64), HSTART = 50 ($32), VSTOP = 120 ($78)
    sprites.set_pos(0, (0x64 << 8) | 0x32);
    sprites.set_ctl(0, (0x78 << 8) | 0x80); // VSTOP=120, ATTACH=1

    let ch0 = &sprites.channels[0];
    assert_eq!(ch0.vstart(), 100);
    assert_eq!(ch0.vstop(), 120);
    assert!(ch0.is_attached());

    sprites.set_data(0, 0xAAAA, 0x5555);
    assert!(sprites.channels[0].is_armed);

    sprites.reset();
    assert_eq!(sprites.channels[0].vstart(), 0);
    assert!(!sprites.channels[0].is_armed);
}
