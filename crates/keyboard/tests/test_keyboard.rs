#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use keyboard::{
    Keyboard, KeyboardTransmissionState, KEYBOARD_BUFFER_CAPACITY, SCANCODE_BUFFER_OVERFLOW,
    SCANCODE_CAPS_LOCK, SCANCODE_CTRL, SCANCODE_L_AMIGA, SCANCODE_POWERUP_STREAM_END,
    SCANCODE_POWERUP_STREAM_START, SCANCODE_R_AMIGA,
};

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

#[test]
fn test_keyboard_fifo_queue_and_overflow() {
    let mut kbd = Keyboard::new();
    assert!(!kbd.has_pending_scancodes());
    assert_eq!(kbd.dequeue_scancode(), None);

    // Enqueue up to capacity
    for i in 0..KEYBOARD_BUFFER_CAPACITY {
        kbd.enqueue_scancode(i as u8);
        assert!(kbd.has_pending_scancodes());
    }
    assert_eq!(kbd.queue_len, KEYBOARD_BUFFER_CAPACITY);

    // Enqueue beyond capacity triggers SCANCODE_BUFFER_OVERFLOW on newest slot
    kbd.enqueue_scancode(0xFF);
    assert_eq!(kbd.queue_len, KEYBOARD_BUFFER_CAPACITY);

    // Dequeue first items
    for i in 0..(KEYBOARD_BUFFER_CAPACITY - 1) {
        assert_eq!(kbd.dequeue_scancode(), Some(i as u8));
    }
    // Final slot must be the buffer overflow warning code
    assert_eq!(kbd.dequeue_scancode(), Some(SCANCODE_BUFFER_OVERFLOW));
    assert!(!kbd.has_pending_scancodes());
    assert_eq!(kbd.dequeue_scancode(), None);
}

#[test]
fn test_keyboard_caps_lock_toggle_semantics() {
    let mut kbd = Keyboard::new();
    assert!(!kbd.caps_lock_active);

    // Caps lock toggle ON: transmits ($62 << 1) & 0xFE = $C4 with bit 7 = 0
    kbd.key_down(SCANCODE_CAPS_LOCK);
    assert!(kbd.caps_lock_active);
    let on_code = (SCANCODE_CAPS_LOCK << 1) & 0xFE;
    assert_eq!(kbd.current_scancode, Some(on_code));

    // Key up for Caps Lock does not transmit
    kbd.current_scancode = None;
    kbd.key_up(SCANCODE_CAPS_LOCK);
    assert_eq!(kbd.current_scancode, None);

    // Caps lock toggle OFF: transmits ($62 << 1) | 0x01
    kbd.key_down(SCANCODE_CAPS_LOCK);
    assert!(!kbd.caps_lock_active);
    let off_code = ((SCANCODE_CAPS_LOCK << 1) & 0xFE) | 0x01;
    assert_eq!(kbd.current_scancode, Some(off_code));
}

#[test]
fn test_keyboard_step_handshake_state_machine() {
    let mut kbd = Keyboard::new();
    assert_eq!(kbd.transmission_state, KeyboardTransmissionState::Idle);

    // No keys queued -> step returns None
    assert_eq!(kbd.step(false), None);

    // Enqueue a key
    kbd.key_down(0x10);
    let expected = (0x10 << 1) & 0xFE;

    // Step should dequeue and transition to WaitingHandshake
    assert_eq!(kbd.step(false), Some(expected));
    assert_eq!(
        kbd.transmission_state,
        KeyboardTransmissionState::WaitingHandshake
    );

    // Subsequent step without handshake returns None
    assert_eq!(kbd.step(false), None);

    // Handshake clears WaitingHandshake back to Idle
    assert_eq!(kbd.step(true), None);
    assert_eq!(kbd.transmission_state, KeyboardTransmissionState::Idle);
}

#[test]
fn test_keyboard_powerup_stream_and_reset() {
    let mut kbd = Keyboard::new();
    let held_keys = [0x15, 0x22];
    kbd.queue_powerup_stream(&held_keys);

    assert_eq!(kbd.dequeue_scancode(), Some(SCANCODE_POWERUP_STREAM_START));
    assert_eq!(kbd.dequeue_scancode(), Some((0x15 << 1) & 0xFE));
    assert_eq!(kbd.dequeue_scancode(), Some((0x22 << 1) & 0xFE));
    assert_eq!(kbd.dequeue_scancode(), Some(SCANCODE_POWERUP_STREAM_END));

    // Reset clears everything
    kbd.key_down(0x30);
    kbd.reset();
    assert_eq!(kbd.current_scancode, None);
    assert_eq!(kbd.queue_len, 0);
    assert_eq!(kbd.transmission_state, KeyboardTransmissionState::Idle);
    assert!(!kbd.reset_line_asserted);
}
