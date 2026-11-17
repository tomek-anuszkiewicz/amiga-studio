#![allow(clippy::unwrap_used, clippy::panic)]
mod common;

use common::keyboard::{key, policy};
use gui::keyboard_input::{KeyCommand, KeyRoute, KeyboardRouter};

#[test]
fn physical_sides_keypad_and_command_edges_remain_distinct() {
    let policy = policy();
    let mut router = KeyboardRouter::default();
    for (physical, code) in [
        ("ShiftLeft", 0x60),
        ("ShiftRight", 0x61),
        ("Digit0", 0x0a),
        ("Numpad0", 0x0f),
        ("Enter", 0x44),
        ("NumpadEnter", 0x43),
    ] {
        for pressed in [true, false] {
            assert_eq!(
                router.route(&key(physical, pressed), &policy),
                KeyRoute::Amiga { code, pressed }
            );
        }
    }
    for pressed in [true, false] {
        assert_eq!(
            router.route(&key("F12", pressed), &policy),
            KeyRoute::Command {
                action: KeyCommand::ToggleView,
                pressed
            }
        );
    }
    router.route(&key("ControlLeft", true), &policy);
    assert_eq!(
        router.route(&key("KeyS", true), &policy),
        KeyRoute::Command {
            action: KeyCommand::SaveState,
            pressed: true
        }
    );
    router.route(&key("ControlLeft", false), &policy);
    assert_eq!(
        router.route(&key("KeyS", false), &policy),
        KeyRoute::Command {
            action: KeyCommand::SaveState,
            pressed: false
        }
    );
}

#[test]
fn text_focus_capture_pause_and_reacquisition_gate_guest_input() {
    for (field, expected) in [
        ("text", "gui_text_focus"),
        ("capture", "capture_disabled"),
        ("pause", "pause_pending"),
        ("focus", "window_inactive"),
    ] {
        let mut policy = policy();
        match field {
            "text" => policy.text_focus = true,
            "capture" => policy.captured = false,
            "pause" => policy.pause_pending = true,
            _ => policy.focused = false,
        }
        let mut router = KeyboardRouter::default();
        assert_eq!(
            router.route(&key("KeyB", true), &policy),
            KeyRoute::Rejected(expected)
        );
        router.release();
        policy = common::keyboard::policy();
        assert_eq!(
            router.route(&key("KeyB", true), &policy),
            KeyRoute::Amiga {
                code: 0x35,
                pressed: true
            }
        );
    }
    let mut router = KeyboardRouter::default();
    let mut policy = policy();
    policy.captured = false;
    policy.running = false;
    assert_eq!(
        router.route(&key("F5", true), &policy),
        KeyRoute::Command {
            action: KeyCommand::Pause,
            pressed: true
        }
    );
    let mut synthetic = key("KeyB", true);
    synthetic.synthetic = true;
    assert_eq!(
        router.route(&synthetic, &policy),
        KeyRoute::Rejected("synthetic_focus_press")
    );
}
