use keyboard::{Keyboard, SCANCODE_CTRL, SCANCODE_L_AMIGA, SCANCODE_R_AMIGA};

#[test]
fn test_keyboard_scancode_and_reset() {
    let mut kbd = Keyboard::new();
    assert!(!kbd.poll_reset());

    // Press key 'A' ($20)
    kbd.key_down(0x20);
    assert_eq!(kbd.current_scancode, Some(0x40)); // ($20 << 1) | 0

    kbd.key_up(0x20);
    assert_eq!(kbd.current_scancode, Some(0x41)); // ($20 << 1) | 1

    // Trigger Ctrl-Amiga-Amiga
    kbd.key_down(SCANCODE_CTRL);
    kbd.key_down(SCANCODE_L_AMIGA);
    assert!(!kbd.reset_line_asserted);
    kbd.key_down(SCANCODE_R_AMIGA);
    assert!(kbd.reset_line_asserted);
    assert!(kbd.poll_reset());
    assert!(!kbd.reset_line_asserted);
}
