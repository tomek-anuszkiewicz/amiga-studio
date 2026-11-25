#![allow(clippy::unwrap_used, clippy::panic)]
mod common;

use common::keyboard::{key, policy, translate};
use gui::host_input::{HostInputEvent, KeyboardEvent};
use gui::input_mapping::InputMapping;
use gui::keyboard_input::{KeyCommand, KeyRoute, KeyboardRouter};

#[test]
fn physical_sides_keypad_and_command_edges_remain_distinct() {
    let mut router = KeyboardRouter::default();
    let mut mapping = InputMapping::default();
    for (physical, code) in [
        ("ShiftLeft", 0x60),
        ("ShiftRight", 0x61),
        ("Digit0", 0x0a),
        ("Numpad0", 0x0f),
        ("Enter", 0x44),
        ("NumpadEnter", 0x43),
        ("MetaLeft", 0x66),
        ("MetaRight", 0x67),
    ] {
        for pressed in [true, false] {
            assert_eq!(
                translate(&mut router, &mut mapping, physical, pressed, true),
                Some(HostInputEvent::Key { code, pressed })
            );
        }
    }
    let policy = policy();
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
fn accepted_frontend_bindings_without_amiga_positions_dispatch_both_edges() {
    let catalog = gui::player_config::ConfigurationCatalog::default();
    let mut configuration = catalog.selected().unwrap().clone();
    let bindings = &mut configuration.controls.host.frontend;
    bindings.release = "End".into();
    bindings.pause = "Home".into();
    bindings.save = "F11".into();
    bindings.load = "PageDown".into();
    assert!(catalog.validate(&configuration).is_ok());
    let mut policy = policy();
    policy.developer = false;
    policy.bindings = configuration.controls.host.frontend;
    for (physical, action) in [
        ("End", KeyCommand::Release),
        ("Home", KeyCommand::Pause),
        ("F11", KeyCommand::QuickSave),
        ("PageDown", KeyCommand::QuickLoad),
    ] {
        assert_eq!(gui::host_keys::amiga_key_code(physical), None);
        let mut router = KeyboardRouter::default();
        for pressed in [true, false] {
            assert_eq!(
                router.route(&key(physical, pressed), &policy),
                KeyRoute::Command { action, pressed },
                "{physical} pressed={pressed}"
            );
        }
    }
}

#[test]
fn gui_ownership_pause_and_reacquisition_gate_guest_input() {
    for (field, expected) in [
        ("editor or dialog", "gui_owns_input"),
        ("pause", "pause_pending"),
        ("focus", "window_inactive"),
        ("session", "not_session_screen"),
        ("launch", "pending_launch"),
        ("stopped", "machine_not_running"),
        ("unmapped", "unmapped_physical_key"),
    ] {
        let mut policy = policy();
        match field {
            "editor or dialog" => policy.gui_owns_input = true,
            "pause" => policy.pause_pending = true,
            "session" => policy.session = false,
            "launch" => policy.pending_launch = true,
            "stopped" => policy.running = false,
            "unmapped" => {}
            _ => policy.focused = false,
        }
        let mut router = KeyboardRouter::default();
        assert_eq!(
            router.route(
                &key(
                    if field == "unmapped" {
                        "PrintScreen"
                    } else {
                        "KeyB"
                    },
                    true
                ),
                &policy
            ),
            KeyRoute::Rejected(expected)
        );
        router.release();
        policy = common::keyboard::policy();
        assert_eq!(
            router.route(&key("KeyB", true), &policy),
            KeyRoute::Gameplay(KeyboardEvent::Key {
                position: "KeyB".into(),
                pressed: true
            })
        );
    }
    let mut router = KeyboardRouter::default();
    let mut policy = policy();
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

#[test]
fn standard_commands_accept_both_modifier_sides_and_keypad_aliases_without_capture() {
    let mut policy = policy();
    policy.running = false;
    for modifier in ["ControlLeft", "ControlRight", "MetaLeft", "MetaRight"] {
        for (physical, action) in [
            ("KeyO", KeyCommand::LoadBinary),
            ("KeyS", KeyCommand::SaveState),
            ("KeyL", KeyCommand::LoadState),
            ("Equal", KeyCommand::ZoomIn),
            ("NumpadAdd", KeyCommand::ZoomIn),
            ("Minus", KeyCommand::ZoomOut),
            ("NumpadSubtract", KeyCommand::ZoomOut),
            ("Digit0", KeyCommand::NormalZoom),
            ("Numpad0", KeyCommand::NormalZoom),
        ] {
            let mut router = KeyboardRouter::default();
            router.route(&key(modifier, true), &policy);
            assert_eq!(
                router.route(&key(physical, true), &policy),
                KeyRoute::Command {
                    action,
                    pressed: true
                }
            );
            router.route(&key(modifier, false), &policy);
            assert_eq!(
                router.route(&key(physical, false), &policy),
                KeyRoute::Command {
                    action,
                    pressed: false
                }
            );
        }
    }
}

#[test]
fn recognized_command_repeats_are_consumed_without_a_second_press() {
    let mut policy = policy();
    policy.bindings.release = "Enter".into();
    let mut router = KeyboardRouter::default();
    let mut enter = key("Enter", true);
    assert_eq!(
        router.route(&enter, &policy),
        KeyRoute::Command {
            action: KeyCommand::Release,
            pressed: true
        },
    );
    enter.repeat = true;
    assert_eq!(router.route(&enter, &policy), KeyRoute::Consumed);
    assert_eq!(
        router.route(&key("Enter", false), &policy),
        KeyRoute::Command {
            action: KeyCommand::Release,
            pressed: false
        },
    );
}

#[cfg(windows)]
#[test]
fn windows_logo_is_a_guest_modifier_only_during_native_capture() {
    let mut captured = policy();
    captured.native_mouse_captured = true;
    let mut router = KeyboardRouter::default();
    for position in ["MetaLeft", "MetaRight"] {
        assert_eq!(
            router.route(&key(position, true), &captured),
            KeyRoute::Gameplay(KeyboardEvent::Key {
                position: position.into(),
                pressed: true
            })
        );
    }
    for pressed in [true, false] {
        assert_eq!(
            router.route(&key("KeyS", pressed), &captured),
            KeyRoute::Gameplay(KeyboardEvent::Key {
                position: "KeyS".into(),
                pressed
            })
        );
    }
    assert!(matches!(
        router.route(&key("ControlRight", true), &captured),
        KeyRoute::Gameplay(_)
    ));
    assert_eq!(
        router.route(&key("KeyS", true), &captured),
        KeyRoute::Command {
            action: KeyCommand::SaveState,
            pressed: true
        }
    );
    assert_eq!(
        router.route(&key("F12", true), &captured),
        KeyRoute::Command {
            action: KeyCommand::ToggleView,
            pressed: true
        }
    );
    let free = policy();
    for position in ["MetaLeft", "MetaRight"] {
        assert_eq!(
            router.route(&key(position, false), &free),
            KeyRoute::Gameplay(KeyboardEvent::Key {
                position: position.into(),
                pressed: false
            })
        );
        let mut router = KeyboardRouter::default();
        router.route(&key(position, true), &free);
        assert_eq!(
            router.route(&key("KeyS", true), &free),
            KeyRoute::Command {
                action: KeyCommand::SaveState,
                pressed: true
            }
        );
    }
}
