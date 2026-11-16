#![allow(clippy::unwrap_used, clippy::panic)]

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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_physical_modifiers_repeats_and_mouse_edges_preserve_their_meaning() {
    use gui::host_native_input::NativeEvents;
    use rdev::{Button, EventType, Key};
    let mut events = NativeEvents::default();
    let key = |code, pressed| Some(HostInputEvent::Key { code, pressed });
    assert_eq!(
        events.translate(EventType::KeyPress(Key::ShiftRight), true),
        key(0x61, true)
    );
    assert_eq!(
        events.translate(EventType::KeyPress(Key::ShiftRight), true),
        None
    );
    assert_eq!(
        events.translate(EventType::KeyRelease(Key::ShiftRight), true),
        key(0x61, false)
    );
    assert_eq!(
        events.translate(EventType::KeyPress(Key::ControlLeft), true),
        key(0x63, true)
    );
    assert_eq!(
        events.translate(EventType::KeyPress(Key::ControlRight), true),
        key(0x63, true)
    );
    assert_eq!(
        events.translate(EventType::KeyRelease(Key::ControlLeft), true),
        key(0x63, true)
    );
    assert_eq!(
        events.translate(EventType::KeyRelease(Key::ControlRight), true),
        key(0x63, false)
    );
    assert_eq!(
        events.translate(EventType::MouseMove { x: 100.0, y: 200.0 }, false),
        None
    );
    assert_eq!(
        events.translate(EventType::MouseMove { x: 103.0, y: 198.0 }, true),
        Some(HostInputEvent::MouseMotion {
            port: 0,
            dx: 3,
            dy: -2
        })
    );
    for (button, index) in [(Button::Left, 0), (Button::Right, 1)] {
        for pressed in [true, false] {
            let raw = if pressed {
                EventType::ButtonPress(button)
            } else {
                EventType::ButtonRelease(button)
            };
            assert_eq!(
                events.translate(raw, true),
                Some(HostInputEvent::MouseButtons {
                    port: 0,
                    left: pressed && index == 0,
                    right: pressed && index == 1,
                    middle: pressed && index == 2
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_shortcuts_disabled_capture_and_key_groups_keep_their_contracts() {
    use gui::host_native_input::NativeEvents;
    use rdev::{Button, EventType, Key};
    let mut events = NativeEvents::default();
    assert_eq!(
        events.translate(EventType::KeyPress(Key::KeyA), false),
        None
    );
    assert_eq!(
        events.translate(EventType::KeyRelease(Key::KeyA), false),
        None
    );
    assert_eq!(
        events.translate(EventType::ButtonPress(Button::Left), false),
        None
    );
    events.translate(EventType::KeyPress(Key::ControlLeft), true);
    assert_eq!(
        events.translate(EventType::KeyPress(Key::KeyR), true),
        Some(HostInputEvent::Key {
            code: 0x13,
            pressed: true
        })
    );
    events.translate(EventType::KeyRelease(Key::KeyR), true);
    events.translate(EventType::KeyRelease(Key::ControlLeft), true);
    for (key, code) in [
        (Key::KeyA, 0x20),
        (Key::KeyQ, 0x10),
        (Key::Num1, 0x01),
        (Key::BackQuote, 0x00),
        (Key::IntlBackslash, 0x30),
        (Key::F1, 0x50),
        (Key::KpReturn, 0x43),
        (Key::AltGr, 0x65),
    ] {
        assert_eq!(
            events.translate(EventType::KeyPress(key), true),
            Some(HostInputEvent::Key {
                code,
                pressed: true
            })
        );
        assert_eq!(
            events.translate(EventType::KeyRelease(key), true),
            Some(HostInputEvent::Key {
                code,
                pressed: false
            })
        );
    }
    assert_eq!(
        events.translate(EventType::KeyPress(Key::F2), true),
        Some(HostInputEvent::Key {
            code: 0x51,
            pressed: true
        })
    );
    assert_eq!(
        events.translate(EventType::ButtonPress(Button::Middle), true),
        None
    );
    assert_eq!(
        events.translate(EventType::ButtonRelease(Button::Middle), true),
        None
    );
    assert_eq!(
        events.translate(EventType::ButtonPress(Button::Unknown(9)), true),
        None
    );
    assert_eq!(gui::host_keys::amiga_key_code("UnmappedKey"), None);
}

#[test]
fn full_release_survives_capture_release_and_clears_held_controller_contacts() {
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
        input.pending.push(HostInputEvent::ReleaseCapture {
            joysticks: [None, Some([false, false, false, false, true, false])],
        });
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
fn capture_release_keeps_fresh_controller_edges_after_a_full_release() {
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
    input.pending.push(HostInputEvent::ReleaseCapture {
        joysticks: [None, Some([false, false, false, false, true, false])],
    });
    assert_eq!(input.pending.first(), Some(HostInputEvent::ReleaseAll));
    input.run_slice(&mut session, 3);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x80,
        0
    );
    assert!(input.pending.is_empty());
}

#[test]
fn capture_release_drops_keyboard_joystick_edges_after_a_full_release() {
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
    input.pending.push(HostInputEvent::ReleaseCapture {
        joysticks: [None; 2],
    });
    input.run_slice(&mut session, 3);
    assert_eq!(
        session.machine.game_ports.sample_ciaa_port_a_inputs() & 0x80,
        0x80,
        "A keyboard joystick press must be discarded when capture ends"
    );
    assert!(input.pending.is_empty());
}
