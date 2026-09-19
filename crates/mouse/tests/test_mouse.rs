#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use mouse::Mouse;

#[test]
fn test_mouse_relative_motion_and_joy0dat_encoding() {
    let mut mouse = Mouse::new();
    assert_eq!(mouse.joy_dat(), 0x0000);
    assert_eq!(mouse.x, 0);
    assert_eq!(mouse.y, 0);

    // Forward positive motion
    mouse.move_rel(42, 105);
    assert_eq!(mouse.x, 42);
    assert_eq!(mouse.y, 105);
    // Y in bits 15-8, X in bits 7-0
    assert_eq!(mouse.joy_dat(), (105 << 8) | 42);

    // Further positive accumulation
    mouse.move_rel(10, 20);
    assert_eq!(mouse.x, 52);
    assert_eq!(mouse.y, 125);
    assert_eq!(mouse.joy_dat(), (125 << 8) | 52);
}

#[test]
fn test_mouse_counter_wrapping_overflow_and_underflow() {
    let mut mouse = Mouse::new();

    // Negative motion from zero wraps around 256
    mouse.move_rel(-1, -5);
    assert_eq!(mouse.x, 255);
    assert_eq!(mouse.y, 251);
    assert_eq!(mouse.joy_dat(), (251 << 8) | 255);

    // Positive overflow across 255 boundary
    mouse.move_rel(2, 6);
    assert_eq!(mouse.x, 1);
    assert_eq!(mouse.y, 1);
    assert_eq!(mouse.joy_dat(), (1 << 8) | 1);

    // Large delta motion wrapping
    mouse.move_rel(512, -257); // 512 % 256 == 0, -257 % 256 == -1 == 255
    assert_eq!(mouse.x, 1);
    assert_eq!(mouse.y, 0);
}

#[test]
fn test_mouse_buttons_state_and_isolation_from_joy0dat() {
    let mut mouse = Mouse::new();
    assert!(!mouse.left_button);
    assert!(!mouse.right_button);
    assert!(!mouse.middle_button);

    mouse.set_left_button(true);
    assert!(mouse.left_button);
    assert!(!mouse.right_button);
    assert!(!mouse.middle_button);

    mouse.set_right_button(true);
    assert!(mouse.left_button);
    assert!(mouse.right_button);
    assert!(!mouse.middle_button);

    mouse.set_middle_button(true);
    assert!(mouse.left_button);
    assert!(mouse.right_button);
    assert!(mouse.middle_button);

    // Mouse buttons are read via CIA-A PRA and POTGO, so joy_dat must strictly reflect (Y, X)
    assert_eq!(mouse.joy_dat(), 0x0000);

    mouse.move_rel(15, 30);
    assert_eq!(mouse.joy_dat(), (30 << 8) | 15);

    mouse.set_left_button(false);
    mouse.set_right_button(false);
    mouse.set_middle_button(false);
    assert!(!mouse.left_button);
    assert!(!mouse.right_button);
    assert!(!mouse.middle_button);
}

#[test]
fn test_mouse_reset_and_defaults() {
    let mut mouse = Mouse::new();
    mouse.move_rel(100, 200);
    mouse.set_left_button(true);
    mouse.set_right_button(true);
    mouse.set_middle_button(true);

    assert_eq!(mouse.joy_dat(), (200 << 8) | 100);

    mouse.reset();
    assert_eq!(mouse.x, 0);
    assert_eq!(mouse.y, 0);
    assert_eq!(mouse.joy_dat(), 0x0000);
    assert!(!mouse.left_button);
    assert!(!mouse.right_button);
    assert!(!mouse.middle_button);

    assert_eq!(mouse, Mouse::default());
}
