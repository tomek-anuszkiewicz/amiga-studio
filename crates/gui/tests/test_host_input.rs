#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use config::{A500Config, VideoStandard};
use gui::host_commands::{HostCommand, HostRequest};
use gui::host_input::{HostInputEvent, InputConsumer, InputQueue, INPUT_QUEUE_CAPACITY};
use gui::host_owner::MachineOwner;
use gui::host_video::VideoExchange;
use gui::DebuggerSession;

fn session(standard: VideoStandard) -> Box<DebuggerSession> {
    let mut session = Box::new(DebuggerSession::from_config(A500Config::standard_1mb(
        standard,
    )));
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    session.set_debugger_enabled(false);
    session
}

#[test]
fn queue_preserves_edges_and_opposing_motion_and_releases_on_overflow() {
    let events = [
        HostInputEvent::Key {
            code: 0x20,
            pressed: true,
        },
        HostInputEvent::MouseMotion {
            port: 0,
            dx: 4,
            dy: -2,
        },
        HostInputEvent::MouseMotion {
            port: 0,
            dx: -4,
            dy: 2,
        },
        HostInputEvent::Key {
            code: 0x20,
            pressed: false,
        },
    ];
    let mut queue = InputQueue::default();
    for event in events {
        queue.push(event);
    }
    for event in events {
        assert_eq!(queue.pop(), Some(event));
    }
    for _ in 0..=INPUT_QUEUE_CAPACITY {
        queue.push(events[0]);
    }
    assert_eq!(queue.overflow_count, 1);
    assert_eq!(queue.pop(), Some(HostInputEvent::ReleaseAll));
    assert!(queue.is_empty());
}

#[test]
fn rapid_buttons_reach_distinct_cck_samples_and_new_events_survive_release() {
    let mut session = session(VideoStandard::Pal);
    session.toggle_run();
    let mut input = InputConsumer::default();
    for event in [
        HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: true,
            middle: true,
        },
        HostInputEvent::MouseButtons {
            port: 0,
            left: false,
            right: false,
            middle: false,
        },
    ] {
        input.pending.push(event);
    }
    input.run_slice(&mut session, 1);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x40, 0);
    assert_eq!(session.machine.paula.pins.pot_input_high & 0x0500, 0);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x40,
        0
    );
    assert_eq!(session.machine.game_ports.potgor() & 0x0500, 0);
    input.run_slice(&mut session, 1);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x40, 0x40);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x40,
        0x40
    );
    input.pending.push(HostInputEvent::ReleaseAll);
    input.pending.push(HostInputEvent::MouseButtons {
        port: 0,
        left: true,
        right: false,
        middle: false,
    });
    input.run_slice(&mut session, 2);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x40,
        0
    );
}

#[test]
fn delivery_boundaries_are_spaced_and_input_arrivals_do_not_postpone_them() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for interlace in [false, true] {
            for lifecycle in ["initial", "cold", "warm", "keyboard"] {
                let mut source = session(standard);
                match lifecycle {
                    "cold" => source.machine.reset(),
                    "warm" => source.machine.reset_warm(),
                    "keyboard" => {
                        for code in [0x63, 0x66, 0x67] {
                            source.machine.keyboard.key_down(code);
                        }
                        source.machine.step_cycles(2);
                        assert_eq!(source.machine.cck, 0);
                    }
                    _ => {}
                }
                source.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
                if interlace {
                    assert!(source.machine.memory_bus().write_custom_word(
                        config::custom_reg::denise::REGISTER_BPLCON0,
                        config::mask::bplcon0::BIT_LACE,
                        "CPU"
                    ));
                }
                let mut owner = MachineOwner::new(source);
                let video = VideoExchange::new(vec![0; owner.video_storage_len()]);
                owner.publish(&video).unwrap();
                video.try_consume(1, |_, _| ()).unwrap();
                owner.execute(HostRequest {
                    id: 1,
                    generation: 1,
                    command: HostCommand::Run,
                });
                let mut boundaries = Vec::new();
                let mut fields = Vec::new();
                let mut previous_deliveries = 0;
                let mut previous_field = 0;
                for tick in 0..1000 {
                    let field_before = owner.status().field_number;
                    owner.collect_input(HostInputEvent::MouseMotion {
                        port: 0,
                        dx: 1,
                        dy: 0,
                    });
                    owner.advance(tick as f64 / 10_000.0, || 0.0);
                    let status = owner.status();
                    if status.input_deliveries != previous_deliveries {
                        boundaries.push((field_before, status.last_input_cck));
                        assert_eq!(status.input_deliveries, previous_deliveries + 1);
                        previous_deliveries = status.input_deliveries;
                    }
                    if status.field_number != previous_field {
                        fields.push((status.field_number, status.input_deliveries, status.cck));
                        previous_field = status.field_number;
                    }
                    owner.publish(&video).unwrap();
                    video.try_consume(1, |_, _| ()).unwrap();
                }
                assert!(fields.len() >= 4, "{standard:?}: {fields:?}");
                assert_eq!(fields[0].1, 3, "First {standard:?} field: {fields:?}");
                for pair in fields.windows(2) {
                    assert_eq!(pair[1].1 - pair[0].1, 3, "{standard:?}: {fields:?}");
                }
                for field in 0..previous_field {
                    let group: Vec<_> = boundaries
                        .iter()
                        .filter(|(number, _)| *number == field)
                        .map(|(_, cck)| *cck)
                        .collect();
                    assert_eq!(group.len(), 3);
                    let gaps: Vec<_> = group.windows(2).map(|pair| pair[1] - pair[0]).collect();
                    assert!(
                        gaps.iter().max().unwrap() - gaps.iter().min().unwrap() <= 1,
                        "{standard:?}: {gaps:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn guest_beam_restarts_do_not_starve_host_input_and_keyboard_reset_changes_generation() {
    let mut source = session(VideoStandard::Pal);
    source
        .load_binary(
            0x1000,
            &[0x33, 0xfc, 0, 0, 0, 0xdf, 0xf0, 0x2c, 0x60, 0xf6],
            true,
        )
        .unwrap();
    let mut owner = MachineOwner::new(source);
    let video = VideoExchange::new(vec![0; owner.video_storage_len()]);
    owner.publish(&video).unwrap();
    video.try_consume(1, |_, _| ()).unwrap();
    owner.execute(HostRequest {
        id: 1,
        generation: 1,
        command: HostCommand::Run,
    });
    for tick in 0..100 {
        owner.collect_input(HostInputEvent::MouseMotion {
            port: 0,
            dx: 1,
            dy: 0,
        });
        owner.advance(tick as f64 / 1000.0, || 0.0);
    }
    assert_eq!(owner.status().field_number, 0);
    assert!(owner.status().input_deliveries >= 14);
    for code in [0x63, 0x66, 0x67] {
        owner.collect_input(HostInputEvent::Key {
            code,
            pressed: true,
        });
    }
    owner.collect_input(HostInputEvent::MouseButtons {
        port: 0,
        left: true,
        right: true,
        middle: true,
    });
    for tick in 100..110 {
        owner.advance(tick as f64 / 1000.0, || 0.0);
        if owner.status().generation != 1 {
            break;
        }
    }
    assert_eq!(owner.status().generation, 2);
    assert_eq!(owner.status().cck, 0);
}

#[test]
fn window_physical_modifiers_repeats_and_mouse_edges_preserve_their_meaning() {
    use common::keyboard::{button, translate};
    let mut events = gui::keyboard_input::KeyboardRouter::default();
    let key = |code, pressed| Some(HostInputEvent::Key { code, pressed });
    assert_eq!(
        translate(&mut events, "ShiftRight", true, true),
        key(0x61, true)
    );
    assert_eq!(translate(&mut events, "ShiftRight", true, true), None);
    assert_eq!(
        translate(&mut events, "ShiftRight", false, true),
        key(0x61, false)
    );
    for (physical, pressed, guest_pressed) in [
        ("ControlLeft", true, true),
        ("ControlRight", true, true),
        ("ControlLeft", false, true),
        ("ControlRight", false, false),
    ] {
        assert_eq!(
            translate(&mut events, physical, pressed, true),
            key(0x63, guest_pressed)
        );
    }
    let mut pointer = gui::window_pointer::WindowPointer::default();
    let motion = egui::Event::MouseMoved(egui::vec2(3.0, -2.0));
    assert_eq!(pointer.translate(&motion, false), None);
    assert_eq!(
        pointer.translate(&motion, true),
        Some(HostInputEvent::MouseMotion {
            port: 0,
            dx: 3,
            dy: -2
        })
    );
    for (raw, index) in [
        (egui::PointerButton::Primary, 0),
        (egui::PointerButton::Secondary, 1),
    ] {
        for pressed in [true, false] {
            assert_eq!(
                pointer.translate(&button(raw, pressed), true),
                Some(HostInputEvent::MouseButtons {
                    port: 0,
                    left: pressed && index == 0,
                    right: pressed && index == 1,
                    middle: false
                })
            );
        }
    }
}

#[test]
fn admitted_key_releases_and_joystick_events_reach_device_pins() {
    let mut session = session(VideoStandard::Pal);
    session.toggle_run();
    let mut input = InputConsumer::default();
    for (code, pressed) in [(0x63, true), (0x66, true), (0x63, false), (0x67, true)] {
        input.pending.push(HostInputEvent::Key { code, pressed });
    }
    input.pending.push(HostInputEvent::Joystick {
        port: 1,
        up: true,
        down: false,
        left: false,
        right: true,
        fire1: true,
        fire2: true,
    });
    input.run_slice(&mut session, 6);
    assert_eq!(
        session.machine.cck, 6,
        "Released Ctrl must prevent the reset combination"
    );
    assert!(!session.machine.keyboard.pins.reset_line_asserted);
    assert_eq!(session.machine.game_ports.pins.direction_pins[1], 0x06);
    assert_eq!(session.machine.denise.pins.game_port_inputs[1], 0x06);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x80, 0);
    assert_eq!(session.machine.paula.pins.pot_input_high & 0x4000, 0);
    input.pending.push(HostInputEvent::ReleaseAll);
    input.run_slice(&mut session, 1);
    assert_eq!(session.machine.game_ports.pins.direction_pins[1], 0x0f);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x80, 0x80);
    assert_eq!(session.machine.paula.pins.pot_input_high & 0x4000, 0x4000);
}

#[test]
fn a_field_boundary_preserves_the_remaining_ordered_input_for_the_next_slice() {
    let mut session = session(VideoStandard::Pal);
    session.toggle_run();
    let mut completed = session.clone();
    completed.run_frame_slice(debugger::session::MAX_FRAME_SLICE_CCK);
    let boundary = completed.machine.cck;
    session.run_frame_slice(boundary as usize - 1);
    let mut input = InputConsumer::default();
    for left in [true, false] {
        input.pending.push(HostInputEvent::MouseButtons {
            port: 0,
            left,
            right: false,
            middle: false,
        });
    }
    input.run_slice(&mut session, 8);
    assert_eq!(session.machine.cck, boundary);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x40, 0);
    assert!(!input.pending.is_empty());
    input.run_slice(&mut session, 1);
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0x40, 0x40);
    assert!(input.pending.is_empty());
}

#[test]
fn window_shortcuts_key_groups_and_uncaptured_pointer_keep_their_contracts() {
    use common::keyboard::{button, translate};
    let mut events = gui::keyboard_input::KeyboardRouter::default();
    let mut pointer = gui::window_pointer::WindowPointer::default();
    assert_eq!(
        pointer.translate(&button(egui::PointerButton::Primary, true), false),
        None
    );
    translate(&mut events, "ControlLeft", true, true);
    assert_eq!(
        translate(&mut events, "KeyR", true, true),
        Some(HostInputEvent::Key {
            code: 0x13,
            pressed: true
        })
    );
    translate(&mut events, "KeyR", false, true);
    translate(&mut events, "ControlLeft", false, true);
    for (physical, code) in [
        ("KeyA", 0x20),
        ("KeyQ", 0x10),
        ("Digit1", 0x01),
        ("Backquote", 0x00),
        ("IntlBackslash", 0x30),
        ("F1", 0x50),
        ("NumpadEnter", 0x43),
        ("AltRight", 0x65),
        ("F2", 0x51),
    ] {
        for pressed in [true, false] {
            assert_eq!(
                translate(&mut events, physical, pressed, true),
                Some(HostInputEvent::Key { code, pressed })
            );
        }
    }
    for raw in [egui::PointerButton::Middle, egui::PointerButton::Extra1] {
        for pressed in [true, false] {
            assert_eq!(pointer.translate(&button(raw, pressed), true), None);
        }
    }
    assert_eq!(gui::host_keys::amiga_key_code("UnmappedKey"), None);
}

#[test]
fn full_release_survives_mouse_release_and_clears_held_controller_contacts() {
    for overflow in [false, true] {
        let mut session = session(VideoStandard::Pal);
        session.toggle_run();
        let mut input = InputConsumer::default();
        input.pending.push(HostInputEvent::Joystick {
            port: 1,
            up: false,
            down: false,
            left: false,
            right: false,
            fire1: true,
            fire2: false,
        });
        input.run_slice(&mut session, 1);
        assert_eq!(
            session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x80,
            0
        );
        if overflow {
            for _ in 0..=INPUT_QUEUE_CAPACITY {
                input.pending.push(HostInputEvent::Key {
                    code: 0x20,
                    pressed: true,
                });
            }
            assert_eq!(input.pending.overflow_count, 1);
        } else {
            input.pending.push(HostInputEvent::ReleaseAll);
        }
        input.pending.push(HostInputEvent::ReleaseMouse { port: 0 });
        assert_eq!(input.pending.first(), Some(HostInputEvent::ReleaseAll));
        input.run_slice(&mut session, 1);
        assert_eq!(
            session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x80,
            0x80
        );
        assert!(input.pending.is_empty());
    }
}

#[test]
fn mouse_release_keeps_fresh_keys_and_controller_edges_after_a_full_release() {
    let mut session = session(VideoStandard::Pal);
    session.toggle_run();
    let mut input = InputConsumer::default();
    input.pending.push(HostInputEvent::ReleaseAll);
    for pressed in [false, true] {
        input.pending.push(HostInputEvent::Joystick {
            port: 1,
            up: false,
            down: false,
            left: false,
            right: false,
            fire1: pressed,
            fire2: false,
        });
    }
    input.pending.push(HostInputEvent::Key {
        code: 0x20,
        pressed: true,
    });
    input.pending.push(HostInputEvent::ReleaseMouse { port: 0 });
    assert_eq!(input.pending.first(), Some(HostInputEvent::ReleaseAll));
    input.run_slice(&mut session, 4);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x80,
        0
    );
    let keyboard = serde_json::to_value(&session.machine.keyboard).unwrap();
    assert_ne!(keyboard["held"][0].as_u64().unwrap() & (1 << 0x20), 0);
    assert!(input.pending.is_empty());
}

#[test]
fn mouse_release_discards_stale_motion_and_buttons_but_keeps_joystick_edges() {
    let mut session = session(VideoStandard::Pal);
    session.toggle_run();
    let mut input = InputConsumer::default();
    input.pending.push(HostInputEvent::ReleaseAll);
    input.pending.push(HostInputEvent::Joystick {
        port: 1,
        up: false,
        down: false,
        left: false,
        right: false,
        fire1: true,
        fire2: false,
    });
    input.pending.push(HostInputEvent::MouseMotion {
        port: 0,
        dx: 20,
        dy: 20,
    });
    input.pending.push(HostInputEvent::MouseButtons {
        port: 0,
        left: true,
        right: true,
        middle: false,
    });
    input.pending.push(HostInputEvent::ReleaseMouse { port: 0 });
    input.run_slice(&mut session, 3);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0xc0,
        0x40
    );
    assert_eq!(session.machine.denise.joy0dat(), 0);
    assert!(input.pending.is_empty());
}

#[test]
fn repeated_mouse_release_on_second_port_freezes_its_quadrature_without_releasing_joystick() {
    use debugger::launch::DeviceKind;
    let mut session = session(VideoStandard::Pal);
    debugger::launch::reconcile_ports(&mut session, [DeviceKind::Joystick, DeviceKind::Mouse]);
    session.toggle_run();
    let mut input = InputConsumer::default();
    input.pending.push(HostInputEvent::Joystick {
        port: 0,
        up: false,
        down: false,
        left: false,
        right: false,
        fire1: true,
        fire2: false,
    });
    input.pending.push(HostInputEvent::MouseButtons {
        port: 1,
        left: true,
        right: true,
        middle: false,
    });
    input.pending.push(HostInputEvent::MouseMotion {
        port: 1,
        dx: 100,
        dy: 0,
    });
    input.run_slice(&mut session, 3);
    let before = serde_json::to_value(session.machine.game_ports.state.port2).unwrap();
    assert_eq!(before["Mouse"]["pending_x"], 99);
    assert_ne!(before["Mouse"]["phase_x"], 0);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0xc0,
        0
    );

    let release = HostInputEvent::ReleaseMouse { port: 1 };
    input.pending.push(release);
    input.pending.push(release);
    input.run_slice(&mut session, 4);
    let after = serde_json::to_value(session.machine.game_ports.state.port2).unwrap();
    assert_eq!(after["Mouse"]["phase_x"], before["Mouse"]["phase_x"]);
    assert_eq!(after["Mouse"]["pending_x"], 0);
    assert_eq!(after["Mouse"]["left_button"], false);
    assert_eq!(after["Mouse"]["right_button"], false);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0xc0,
        0x80
    );
}
