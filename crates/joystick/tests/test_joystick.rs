#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use joystick::Joystick;

#[test]
fn test_joystick_directional_encoding_all_quadrants() {
    let mut joy = Joystick::new();
    assert_eq!(joy.joy_dat(), 0x0000);

    // 1. Cardinal directions:
    // UP only: Bit 8 = 1, Bit 9 = 0 -> 0x0100
    joy.set_directions(true, false, false, false);
    assert_eq!(joy.joy_dat(), 0x0100);

    // DOWN only: Bit 0 = 1 ^ 0 = 1, Bit 1 = 0 -> 0x0001
    joy.set_directions(false, true, false, false);
    assert_eq!(joy.joy_dat(), 0x0001);

    // LEFT only: Bit 9 = 1, Bit 8 = 0 ^ 1 = 1 -> 0x0300
    joy.set_directions(false, false, true, false);
    assert_eq!(joy.joy_dat(), 0x0300);

    // RIGHT only: Bit 1 = 1, Bit 0 = 0 ^ 1 = 1 -> 0x0003
    joy.set_directions(false, false, false, true);
    assert_eq!(joy.joy_dat(), 0x0003);

    // 2. Diagonal directions:
    // UP + LEFT: Bit 9 = 1, Bit 8 = 1 ^ 1 = 0 -> 0x0200
    joy.set_directions(true, false, true, false);
    assert_eq!(joy.joy_dat(), 0x0200);

    // UP + RIGHT: Bit 8 = 1 ^ 0 = 1, Bit 1 = 1, Bit 0 = 0 ^ 1 = 1 -> 0x0103
    joy.set_directions(true, false, false, true);
    assert_eq!(joy.joy_dat(), 0x0103);

    // DOWN + LEFT: Bit 9 = 1, Bit 8 = 0 ^ 1 = 1, Bit 0 = 1 ^ 0 = 1 -> 0x0301
    joy.set_directions(false, true, true, false);
    assert_eq!(joy.joy_dat(), 0x0301);

    // DOWN + RIGHT: Bit 1 = 1, Bit 0 = 1 ^ 1 = 0 -> 0x0002
    joy.set_directions(false, true, false, true);
    assert_eq!(joy.joy_dat(), 0x0002);
}

#[test]
fn test_joystick_fire_buttons_and_isolation_from_joydat() {
    let mut joy = Joystick::new();
    assert!(!joy.fire1);
    assert!(!joy.fire2);

    joy.set_fire1(true);
    assert!(joy.fire1);
    assert!(!joy.fire2);
    // Fire buttons are decoded via CIA-A PRA and POTGO, so joy_dat must remain 0
    assert_eq!(joy.joy_dat(), 0x0000);

    joy.set_fire2(true);
    assert!(joy.fire1);
    assert!(joy.fire2);
    assert_eq!(joy.joy_dat(), 0x0000);

    joy.set_fire1(false);
    assert!(!joy.fire1);
    assert!(joy.fire2);

    joy.set_fire2(false);
    assert!(!joy.fire1);
    assert!(!joy.fire2);
}

#[test]
fn test_joystick_reset_and_defaults() {
    let mut joy = Joystick::new();
    joy.set_directions(true, false, true, false);
    joy.set_fire1(true);
    joy.set_fire2(true);

    assert_eq!(joy.joy_dat(), 0x0200);
    assert!(joy.fire1);
    assert!(joy.fire2);

    joy.reset();
    assert_eq!(joy.joy_dat(), 0x0000);
    assert!(!joy.up);
    assert!(!joy.down);
    assert!(!joy.left);
    assert!(!joy.right);
    assert!(!joy.fire1);
    assert!(!joy.fire2);

    assert_eq!(joy, Joystick::default());
}
