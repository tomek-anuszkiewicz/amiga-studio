use joystick::Joystick;

#[test]
fn test_joystick_encoding() {
    let mut joy = Joystick::new();
    assert_eq!(joy.joy_dat(), 0);

    // UP only: Bit 8 = 1, Bit 9 = 0 -> 0x0100
    joy.set_directions(true, false, false, false);
    assert_eq!(joy.joy_dat(), 0x0100);

    // LEFT only: Bit 9 = 1, Bit 8 = 0 ^ 1 = 1 -> 0x0300
    joy.set_directions(false, false, true, false);
    assert_eq!(joy.joy_dat(), 0x0300);

    // UP + LEFT: Bit 9 = 1, Bit 8 = 1 ^ 1 = 0 -> 0x0200
    joy.set_directions(true, false, true, false);
    assert_eq!(joy.joy_dat(), 0x0200);

    // RIGHT only: Bit 1 = 1, Bit 0 = 0 ^ 1 = 1 -> 0x0003
    joy.set_directions(false, false, false, true);
    assert_eq!(joy.joy_dat(), 0x0003);

    joy.reset();
    assert_eq!(joy.joy_dat(), 0);
}
