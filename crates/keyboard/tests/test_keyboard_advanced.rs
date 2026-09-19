use keyboard::{
    Keyboard, KeyboardTransmissionState, SCANCODE_BUFFER_OVERFLOW, SCANCODE_CAPS_LOCK,
    SCANCODE_CTRL, SCANCODE_LOST_SYNC, SCANCODE_L_AMIGA, SCANCODE_POWERUP_STREAM_END,
    SCANCODE_POWERUP_STREAM_START, SCANCODE_RESET_WARNING, SCANCODE_R_AMIGA,
    SCANCODE_SELF_TEST_FAILED,
};

#[test]
fn test_keyboard_fifo_queue_ordering() {
    let mut kbd = Keyboard::new();

    // Type 'A' ($20), 'B' ($35), 'C' ($33)
    kbd.key_down(0x20);
    kbd.key_up(0x20);
    kbd.key_down(0x35);
    kbd.key_up(0x35);

    assert_eq!(kbd.queue_len, 4);

    // Dequeue in FIFO order
    assert_eq!(kbd.dequeue_scancode(), Some((0x20 << 1) & 0xFE)); // A down
    assert_eq!(kbd.dequeue_scancode(), Some(((0x20 << 1) & 0xFE) | 0x01)); // A up
    assert_eq!(kbd.dequeue_scancode(), Some((0x35 << 1) & 0xFE)); // B down
    assert_eq!(kbd.dequeue_scancode(), Some(((0x35 << 1) & 0xFE) | 0x01)); // B up
    assert_eq!(kbd.dequeue_scancode(), None);
    assert_eq!(kbd.queue_len, 0);
}

#[test]
fn test_keyboard_buffer_overflow() {
    let mut kbd = Keyboard::new();

    // Fill the 16-element queue
    for i in 0..16 {
        kbd.key_down(i);
    }
    assert_eq!(kbd.queue_len, 16);

    // 17th keystroke causes buffer overflow flag
    kbd.key_down(0x1F);
    assert_eq!(kbd.queue_len, 16);

    // Dequeue all: last element must be SCANCODE_BUFFER_OVERFLOW
    let mut last = 0;
    while let Some(code) = kbd.dequeue_scancode() {
        last = code;
    }
    assert_eq!(last, SCANCODE_BUFFER_OVERFLOW);
}

#[test]
fn test_caps_lock_toggle_quirk() {
    let mut kbd = Keyboard::new();

    // First press of Caps Lock: turns LED on, transmits $62 (bit 7 = 0)
    kbd.key_down(SCANCODE_CAPS_LOCK);
    assert!(kbd.caps_lock_active);
    assert_eq!(
        kbd.dequeue_scancode(),
        Some((SCANCODE_CAPS_LOCK << 1) & 0xFE)
    );

    // Release of Caps Lock: transmits NOTHING
    kbd.key_up(SCANCODE_CAPS_LOCK);
    assert_eq!(kbd.dequeue_scancode(), None);

    // Second press of Caps Lock: turns LED off, transmits $E2 (bit 7 = 1)
    kbd.key_down(SCANCODE_CAPS_LOCK);
    assert!(!kbd.caps_lock_active);
    assert_eq!(
        kbd.dequeue_scancode(),
        Some(((SCANCODE_CAPS_LOCK << 1) & 0xFE) | 0x01)
    );

    // Release of Caps Lock: transmits NOTHING
    kbd.key_up(SCANCODE_CAPS_LOCK);
    assert_eq!(kbd.dequeue_scancode(), None);
}

#[test]
fn test_ctrl_amiga_amiga_reset_sequence() {
    let mut kbd = Keyboard::new();
    assert!(!kbd.reset_line_asserted);

    kbd.key_down(SCANCODE_CTRL);
    kbd.key_down(SCANCODE_L_AMIGA);
    assert!(!kbd.reset_line_asserted);

    // Pressing 3rd key asserts hardware reset line and queues reset warning ($78)
    kbd.key_down(SCANCODE_R_AMIGA);
    assert!(kbd.reset_line_asserted);

    // Reset warning is in the queue: ($78 << 1) & 0xFE = $F0
    let warning = (SCANCODE_RESET_WARNING << 1) & 0xFE;
    let mut found_warning = false;
    while let Some(code) = kbd.dequeue_scancode() {
        if code == warning {
            found_warning = true;
            break;
        }
    }
    assert!(found_warning);

    // Polling reset clears the flag
    assert!(kbd.poll_reset());
    assert!(!kbd.reset_line_asserted);
}

#[test]
fn test_serial_transmission_handshake_state_machine() {
    let mut kbd = Keyboard::new();
    assert_eq!(kbd.transmission_state, KeyboardTransmissionState::Idle);

    kbd.key_down(0x20); // 'A'

    // Step with no handshake: pops scancode and transitions to WaitingHandshake
    let code = kbd.step(false);
    assert_eq!(code, Some((0x20 << 1) & 0xFE));
    assert_eq!(
        kbd.transmission_state,
        KeyboardTransmissionState::WaitingHandshake
    );

    // Step again without handshake: returns None because it's waiting for handshake
    assert_eq!(kbd.step(false), None);

    // Queue another key while waiting
    kbd.key_down(0x30);

    // Step with handshake (Amiga OS pulled KDAT low): acknowledges previous, pops next!
    let next_code = kbd.step(true);
    assert_eq!(next_code, Some((0x30 << 1) & 0xFE));
    assert_eq!(
        kbd.transmission_state,
        KeyboardTransmissionState::WaitingHandshake
    );

    // Handshake the second key
    kbd.acknowledge();
    assert_eq!(kbd.transmission_state, KeyboardTransmissionState::Idle);
    assert_eq!(kbd.step(false), None);
}

#[test]
fn test_powerup_stream_generation() {
    let mut kbd = Keyboard::new();

    // Simulate boot with left mouse button (raw code 0x68) and 'A' (raw code 0x20) held
    kbd.queue_powerup_stream(&[0x20]);

    assert_eq!(kbd.dequeue_scancode(), Some(SCANCODE_POWERUP_STREAM_START));
    assert_eq!(kbd.dequeue_scancode(), Some((0x20 << 1) & 0xFE));
    assert_eq!(kbd.dequeue_scancode(), Some(SCANCODE_POWERUP_STREAM_END));
    assert_eq!(kbd.dequeue_scancode(), None);
}

#[test]
fn test_keyboard_protocol_codes() {
    assert_eq!(SCANCODE_LOST_SYNC, 0xF9);
    assert_eq!(SCANCODE_SELF_TEST_FAILED, 0xFC);
}
