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
fn text_focus_capture_pause_and_reacquisition_gate_guest_input() {
    for (field, expected) in [
        ("text", "gui_text_focus"),
        ("capture", "capture_disabled"),
        ("pause", "pause_pending"),
        ("focus", "window_inactive"),
        ("session", "not_session_screen"),
        ("launch", "pending_launch"),
        ("stopped", "machine_not_running"),
        ("unmapped", "unmapped_physical_key"),
    ] {
        let mut policy = policy();
        match field {
            "text" => policy.text_focus = true,
            "capture" => policy.captured = false,
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

#[test]
fn standard_commands_accept_both_modifier_sides_and_keypad_aliases_without_capture() {
    let mut policy = policy();
    policy.captured = false;
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_ingress_preserves_metadata_and_bounds_the_queue_with_a_release_marker() {
    let ctx = egui::Context::default();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("window_keyboard_generation"), 41u64));
    let event = gui::keyboard_input::PhysicalKey {
        physical: "SuperLeft".into(),
        logical: "different logical key".into(),
        pressed: true,
        repeat: true,
        synthetic: true,
    };
    gui::window_keyboard::receive_key(&ctx, event.clone());
    type RawKeys = Vec<(String, String, bool, bool, bool, u64)>;
    let raw = ctx
        .data_mut(|data| data.remove_temp::<RawKeys>(egui::Id::new("window_keyboard_events")))
        .unwrap();
    assert_eq!(
        raw,
        vec![(
            "SuperLeft".into(),
            "different logical key".into(),
            true,
            true,
            true,
            41
        )]
    );
    for _ in 0..gui::host_input::INPUT_QUEUE_CAPACITY {
        gui::window_keyboard::receive_key(&ctx, event.clone());
    }
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("window_keyboard_generation"), 42u64));
    gui::window_keyboard::receive_key(&ctx, key("KeyB", false));
    let raw = ctx
        .data_mut(|data| data.remove_temp::<RawKeys>(egui::Id::new("window_keyboard_events")))
        .unwrap();
    assert_eq!(raw.len(), 2);
    assert_eq!(
        raw[0],
        (
            "__overflow__".into(),
            String::new(),
            false,
            false,
            false,
            42
        )
    );
    assert_eq!(
        raw[1],
        ("KeyB".into(), "KeyB".into(), false, false, false, 42)
    );
}
