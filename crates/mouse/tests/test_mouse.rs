use mouse::Mouse;

#[test]
fn test_mouse_motion_and_encoding() {
    let mut mouse = Mouse::new();
    assert_eq!(mouse.joy_dat(), 0x0000);

    mouse.move_rel(10, 25);
    assert_eq!(mouse.x, 10);
    assert_eq!(mouse.y, 25);
    assert_eq!(mouse.joy_dat(), (25 << 8) | 10);

    mouse.set_left_button(true);
    assert!(mouse.left_button);

    mouse.reset();
    assert_eq!(mouse.joy_dat(), 0);
    assert!(!mouse.left_button);
}
