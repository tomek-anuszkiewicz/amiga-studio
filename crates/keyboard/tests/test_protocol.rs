//! HRM Appendix H wire examples and recovery protocol; no CIA shortcut.
#![allow(clippy::panic)]

use keyboard::{Keyboard, SCANCODE_CAPS_LOCK, SCANCODE_CTRL, SCANCODE_L_AMIGA, SCANCODE_R_AMIGA};

fn step(keyboard: &mut Keyboard, host_high: bool) -> bool {
    let previous = keyboard.pins.clock_high;
    keyboard.pins.host_data_high = keyboard.pins.data_high && host_high;
    keyboard.step_cck();
    !previous && keyboard.pins.clock_high
}

fn bits(keyboard: &mut Keyboard, count: usize, limit: usize) -> u8 {
    let mut word = 0;
    let mut received = 0;
    for _ in 0..limit {
        if step(keyboard, true) {
            word = (word << 1) | u8::from(keyboard.pins.data_high);
            received += 1;
            if received == count {
                for _ in 0..100 {
                    assert!(!step(keyboard, true));
                }
                return word;
            }
        }
    }
    panic!("received {received} of {count} bits");
}

fn acknowledge(keyboard: &mut Keyboard, duration: usize) {
    for _ in 0..duration {
        assert!(!step(keyboard, false));
    }
    assert!(!step(keyboard, true));
}

fn boot(keyboard: &mut Keyboard) {
    assert_eq!(bits(keyboard, 1, 1000), 0);
    acknowledge(keyboard, 4);
    assert_eq!(bits(keyboard, 8, 3000), 0x04); // !$FD rotated left
    acknowledge(keyboard, 305); // 85 us for both standards
    assert_eq!(bits(keyboard, 8, 3000), 0x02); // !$FE rotated left
    acknowledge(keyboard, 305);
}

#[test]
fn startup_stream_reports_held_keys_then_rotated_active_low_events() {
    for hz in [3_546_895, 3_579_545] {
        let mut keyboard = Keyboard::new(hz);
        keyboard.key_down(0x35);
        assert_eq!(bits(&mut keyboard, 1, 1000), 0);
        acknowledge(&mut keyboard, 4);
        for expected in [0x04, 0x95, 0x02] {
            assert_eq!(bits(&mut keyboard, 8, 3000), expected);
            acknowledge(&mut keyboard, 305);
        }
        keyboard.key_up(0x35);
        assert_eq!(bits(&mut keyboard, 8, 3000), 0x94);
        acknowledge(&mut keyboard, 305);
        for expected in [0x3b, 0x3a] {
            keyboard.key_down(SCANCODE_CAPS_LOCK);
            keyboard.key_up(SCANCODE_CAPS_LOCK);
            assert_eq!(bits(&mut keyboard, 8, 3000), expected);
            acknowledge(&mut keyboard, 305);
        }
        for _ in 0..3000 {
            assert!(!step(&mut keyboard, true));
        }
    }
}

#[test]
fn handshake_rejects_short_pulses_and_waits_for_release_of_a_held_low() {
    let mut keyboard = Keyboard::new(3_546_895);
    boot(&mut keyboard);
    keyboard.key_down(0x35);
    keyboard.key_up(0x35);
    assert_eq!(bits(&mut keyboard, 8, 3000), 0x95);
    acknowledge(&mut keyboard, 1);
    for _ in 0..3000 {
        assert!(!step(&mut keyboard, true));
    }
    acknowledge(&mut keyboard, 600_000); // Held beyond the 143-ms timeout.
    assert_eq!(bits(&mut keyboard, 8, 3000), 0x94);
}

#[test]
fn timeout_resynchronizes_then_reports_lost_sync_and_retransmits() {
    for hz in [3_546_895, 3_579_545] {
        let mut keyboard = Keyboard::new(hz);
        boot(&mut keyboard);
        keyboard.key_down(0x35);
        assert_eq!(bits(&mut keyboard, 8, 3000), 0x95);
        for _ in 0..400_000 {
            assert!(!step(&mut keyboard, true), "no early resync clock");
        }
        assert_eq!(bits(&mut keyboard, 1, 120_000), 0);
        acknowledge(&mut keyboard, 305);
        assert_eq!(bits(&mut keyboard, 8, 3000), 0x0c); // Logical $F9.
                                                        // Lose the report itself: the original key must still remain pending.
        assert_eq!(bits(&mut keyboard, 1, 520_000), 0);
        acknowledge(&mut keyboard, 305);
        assert_eq!(bits(&mut keyboard, 8, 3000), 0x0c);
        acknowledge(&mut keyboard, 305);
        assert_eq!(bits(&mut keyboard, 8, 3000), 0x95);
    }
}

#[test]
fn ten_code_buffer_reports_overflow_and_a500_reset_is_a_separate_line() {
    let mut keyboard = Keyboard::new(3_546_895);
    boot(&mut keyboard);
    for code in 0..12 {
        keyboard.enqueue_scancode(code);
    }
    for code in 0u8..10 {
        assert_eq!(bits(&mut keyboard, 8, 3000), !code.rotate_left(1));
        acknowledge(&mut keyboard, 305);
    }
    assert_eq!(bits(&mut keyboard, 8, 3000), 0x0a); // Logical $FA.
    keyboard.key_down(SCANCODE_CTRL);
    keyboard.key_down(SCANCODE_L_AMIGA);
    keyboard.key_down(SCANCODE_R_AMIGA);
    assert!(keyboard.pins.reset_line_asserted);
    keyboard.key_up(SCANCODE_CTRL);
    assert!(!keyboard.pins.reset_line_asserted);
    keyboard.reset();
    assert_eq!(bits(&mut keyboard, 1, 1000), 0);
}
