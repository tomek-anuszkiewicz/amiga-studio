#![allow(clippy::unwrap_used)]
use debugger::launch::{DeviceKind, MachineSetup};
use gui::host_input::{HostInputEvent, InputConsumer};
use gui::input_mapping::{GamepadInfo, InputMapping};
use gui::player_config::{ConfigurationCatalog, InputSource, PortBinding};

fn two_joysticks() -> (ConfigurationCatalog, gui::player_config::NamedConfiguration) {
    let catalog = ConfigurationCatalog::default();
    let mut config = catalog.selected().unwrap().clone();
    config.ports = [
        PortBinding {
            device: DeviceKind::Joystick,
            source: InputSource::Keyboard("arrows".into()),
        },
        PortBinding {
            device: DeviceKind::Joystick,
            source: InputSource::Gamepad {
                identity: "player-two".into(),
                mapping: "arrows".into(),
            },
        },
    ];
    (catalog, config)
}
fn rom() -> Vec<u8> {
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    rom
}
#[test]
fn keyboard_and_selected_gamepad_reach_independent_ports_and_release_both() {
    let (catalog, config) = two_joysticks();
    assert!(catalog.validate(&config).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&config, &catalog);
    mapping.update_devices(vec![
        GamepadInfo {
            identity: "wrong-pad".into(),
            name: "Other".into(),
        },
        GamepadInfo {
            identity: "player-two".into(),
            name: "Selected".into(),
        },
    ]);
    assert!(mapping
        .gamepad("wrong-pad", &[1.0, 0.0], &[true])
        .is_empty());
    mapping.gamepad("player-two", &[0.0, 0.0], &[false; 16]);
    let mut events = mapping.keyboard(0x0f, true); // keyboard fire, consumed
    assert!(events
        .iter()
        .all(|e| !matches!(e, HostInputEvent::Key { .. })));
    events.extend(mapping.gamepad("player-two", &[1.0, 0.0], &[true]));
    let mut session = MachineSetup {
        config: config::A500Config::default(),
        ports: [DeviceKind::Joystick; 2],
        drives: [true; 4],
        rom: rom(),
        disks: std::array::from_fn(|_| None),
    }
    .prepare()
    .unwrap();
    session.set_debugger_enabled(false);
    session.toggle_run();
    let mut input = InputConsumer::default();
    for event in events {
        input.pending.push(event);
    }
    input.run_slice(&mut session, 8);
    assert!(session.machine.game_ports.fire1_port1());
    assert!(session.machine.game_ports.fire1_port2());
    assert_eq!(session.machine.cia_a.pins.port_a_input & 0xc0, 0);
    assert_ne!(
        session.machine.game_ports.pins.direction_pins[0],
        session.machine.game_ports.pins.direction_pins[1]
    );
    input.release(&mut session);
    assert!(!session.machine.game_ports.fire1_port1());
    assert!(!session.machine.game_ports.fire1_port2());
    mapping.release();
    assert!(
        mapping
            .gamepad("player-two", &[1.0, 0.0], &[true])
            .is_empty(),
        "held state must not replay after capture loss"
    );
    mapping.gamepad("player-two", &[0.0, 0.0], &[false; 16]);
    assert!(!mapping
        .gamepad("player-two", &[0.0, 0.0], &[true])
        .is_empty());
    assert_eq!(
        mapping.update_devices(vec![]),
        vec![HostInputEvent::ReleaseAll]
    );
}
#[test]
fn two_keyboard_mappings_consume_only_their_keys_and_detect_conflicts() {
    let (mut catalog, mut config) = two_joysticks();
    let mut second = catalog.mappings[0].clone();
    second.id = "wasd".into();
    second.keys = [0x11, 0x21, 0x20, 0x22, 0x40, 0x33];
    catalog.mappings.push(second);
    config.ports[1].source = InputSource::Keyboard("wasd".into());
    assert!(catalog.validate(&config).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&config, &catalog);
    assert!(matches!(
        mapping.keyboard(0x4c, true).as_slice(),
        [HostInputEvent::Joystick {
            port: 0,
            up: true,
            ..
        }]
    ));
    assert!(matches!(
        mapping.keyboard(0x11, true).as_slice(),
        [HostInputEvent::Joystick {
            port: 1,
            up: true,
            ..
        }]
    ));
    assert_eq!(
        mapping.keyboard(0x10, true),
        vec![HostInputEvent::Key {
            code: 0x10,
            pressed: true
        }]
    );
    assert!(
        mapping.keyboard(0x54, true).is_empty(),
        "F5 belongs to frontend"
    );
    config.ports[1].source = InputSource::Keyboard("arrows".into());
    assert!(catalog.validate(&config).is_err());
}
#[test]
fn ambiguous_controller_ids_never_drive_a_port() {
    let (catalog, config) = two_joysticks();
    let mut mapping = InputMapping::default();
    mapping.configure(&config, &catalog);
    let pad = GamepadInfo {
        identity: "player-two".into(),
        name: "Duplicated".into(),
    };
    mapping.update_devices(vec![pad.clone(), pad]);
    mapping.gamepad("player-two", &[], &[false; 16]);
    assert!(mapping.gamepad("player-two", &[], &[true]).is_empty());
}

#[test]
fn configured_letter_shortcuts_are_consumed_before_guest_keys() {
    let catalog = ConfigurationCatalog::default();
    let mut configuration = catalog.selected().unwrap().clone();
    configuration.frontend.pause = "A".into();
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration, &catalog);
    assert!(mapping.keyboard(0x20, true).is_empty());
    assert!(mapping.keyboard(0x20, false).is_empty());
}

#[test]
fn two_selected_gamepads_keep_player_states_and_frontend_edges_independent() {
    use gui::player_config::{FrontendAction, GamepadShortcut};
    let (catalog, mut configuration) = two_joysticks();
    configuration.ports[0].source = InputSource::Gamepad {
        identity: "player-one".into(),
        mapping: "arrows".into(),
    };
    configuration.gamepad_shortcuts.push(GamepadShortcut {
        identity: "player-two".into(),
        button: 9,
        action: FrontendAction::Pause,
    });
    assert!(catalog.validate(&configuration).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration, &catalog);
    mapping.update_devices(vec![
        GamepadInfo {
            identity: "player-one".into(),
            name: "One".into(),
        },
        GamepadInfo {
            identity: "player-two".into(),
            name: "Two".into(),
        },
    ]);
    for id in ["player-one", "player-two"] {
        mapping.gamepad(id, &[0.0, 0.0], &[false; 16]);
    }
    assert!(matches!(
        mapping
            .gamepad("player-one", &[-1.0, 0.0], &[true])
            .as_slice(),
        [HostInputEvent::Joystick {
            port: 0,
            left: true,
            fire1: true,
            ..
        }]
    ));
    assert!(matches!(
        mapping
            .gamepad("player-two", &[0.0, 1.0], &[false, true])
            .as_slice(),
        [HostInputEvent::Joystick {
            port: 1,
            down: true,
            fire2: true,
            ..
        }]
    ));
    mapping.frontend_gamepad("player-two", &[false; 16], true);
    let mut buttons = [false; 16];
    buttons[9] = true;
    mapping.frontend_gamepad("player-two", &buttons, true);
    mapping.frontend_gamepad("player-two", &buttons, true);
    assert_eq!(mapping.actions, vec![FrontendAction::Pause]);
    mapping.actions.clear();
    mapping.frontend_gamepad("player-two", &[false; 16], false);
    mapping.frontend_gamepad("player-two", &buttons, false);
    assert!(mapping.actions.is_empty());
}

#[test]
fn system_mouse_targets_second_port_with_scaling_and_unassigned_input_is_dropped() {
    let catalog = ConfigurationCatalog::default();
    let mut configuration = catalog.configurations[0].clone();
    configuration.ports.swap(0, 1);
    configuration.mouse_sensitivity = 1.5;
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration, &catalog);
    assert_eq!(
        mapping.pointer(HostInputEvent::MouseMotion {
            port: 0,
            dx: 4,
            dy: -2
        }),
        vec![HostInputEvent::MouseMotion {
            port: 1,
            dx: 6,
            dy: -3
        }]
    );
    assert_eq!(
        mapping.pointer(HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: false,
            middle: true
        }),
        vec![HostInputEvent::MouseButtons {
            port: 1,
            left: true,
            right: false,
            middle: false
        }]
    );
    configuration.ports[1].device = DeviceKind::Disconnected;
    configuration.ports[1].source = InputSource::None;
    mapping.configure(&configuration, &catalog);
    assert!(mapping
        .pointer(HostInputEvent::MouseMotion {
            port: 0,
            dx: 9,
            dy: 4
        })
        .is_empty());
}

#[test]
fn keyboard_aliases_release_the_guest_key_only_after_both_physical_keys_release() {
    let catalog = ConfigurationCatalog::default();
    let mut configuration = catalog.configurations[0].clone();
    configuration.keyboard = vec![(0x20, 0x63), (0x21, 0x63)];
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration, &catalog);
    for (code, pressed, held) in [
        (0x20, true, true),
        (0x21, true, true),
        (0x20, false, true),
        (0x21, false, false),
    ] {
        assert_eq!(
            mapping.keyboard(code, pressed),
            vec![HostInputEvent::Key {
                code: 0x63,
                pressed: held
            }]
        );
    }
}

#[test]
fn configuration_rejects_ambiguous_devices_keys_and_frontend_actions() {
    use gui::player_config::{FrontendAction, GamepadShortcut};
    fn rejected(
        change: impl FnOnce(&mut ConfigurationCatalog, &mut gui::player_config::NamedConfiguration),
        expected: &str,
    ) {
        let mut catalog = ConfigurationCatalog::default();
        let mut configuration = catalog.selected().unwrap().clone();
        change(&mut catalog, &mut configuration);
        assert!(
            catalog
                .validate(&configuration)
                .err()
                .unwrap_or_else(|| panic!("Accepted invalid configuration: {expected}"))
                .contains(expected),
            "{expected}"
        );
    }
    rejected(|_, c| c.drives[0] = false, "DF0");
    rejected(|_, c| c.name.clear(), "configuration name");
    rejected(|_, c| c.ports[1] = c.ports[0].clone(), "one system mouse");
    rejected(|_, c| c.ports[0].source = InputSource::None, "compatible");
    rejected(
        |_, c| c.ports[0] = c.ports[1].clone(),
        "conflicting destinations",
    );
    rejected(
        |_, c| c.ports[1].source = InputSource::Keyboard("missing".into()),
        "existing input mapping",
    );
    let pad = PortBinding {
        device: DeviceKind::Joystick,
        source: InputSource::Gamepad {
            identity: "pad".into(),
            mapping: "arrows".into(),
        },
    };
    rejected(
        |_, c| c.ports = [pad.clone(), pad.clone()],
        "different available controller",
    );
    rejected(
        |catalog, c| {
            c.ports[1] = pad.clone();
            catalog.mappings[0].threshold = 0.05;
        },
        "threshold",
    );
    rejected(
        |_, c| {
            c.ports[1] = pad.clone();
            c.gamepad_shortcuts.push(GamepadShortcut {
                identity: "pad".into(),
                button: 0,
                action: FrontendAction::Pause,
            });
        },
        "conflicting frontend/joystick",
    );
    rejected(
        |_, c| {
            let s = GamepadShortcut {
                identity: "other".into(),
                button: 9,
                action: FrontendAction::Pause,
            };
            c.gamepad_shortcuts = vec![s.clone(), s];
        },
        "conflicting frontend/joystick",
    );
    rejected(|_, c| c.keyboard = vec![(0x20, 0x80)], "7-bit");
    rejected(
        |_, c| c.keyboard = vec![(0x20, 0x21), (0x20, 0x22)],
        "unique physical",
    );
    rejected(|_, c| c.keyboard = vec![(0x4c, 0x20)], "joystick key");
    rejected(
        |_, c| c.frontend.pause = "missing-key".into(),
        "physical frontend key",
    );
    rejected(|_, c| c.frontend.pause = "F12".into(), "reserved");
    rejected(
        |_, c| c.frontend.pause = c.frontend.save.clone(),
        "different keys",
    );
    rejected(
        |_, c| {
            c.frontend.pause = "A".into();
            c.keyboard = vec![(0x20, 0x21)];
        },
        "keyboard override",
    );
    for key in ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"] {
        rejected(|_, c| c.frontend.pause = key.into(), "joystick mapping");
    }
    let catalog = ConfigurationCatalog::default();
    let mut valid = catalog.selected().unwrap().clone();
    valid.ports[1] = pad;
    valid.gamepad_shortcuts.push(GamepadShortcut {
        identity: "pad".into(),
        button: 9,
        action: FrontendAction::Pause,
    });
    assert!(catalog.validate(&valid).is_ok());
}

#[test]
fn optional_release_defaults_to_guest_escape_and_f2_and_reserves_f12() {
    let catalog = ConfigurationCatalog::default();
    let mut configuration = catalog.selected().unwrap().clone();
    assert!(configuration.frontend.release.is_empty());
    assert!(catalog.validate(&configuration).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration, &catalog);
    for code in [0x45, 0x51] {
        assert_eq!(
            mapping.keyboard(code, true),
            vec![HostInputEvent::Key {
                code,
                pressed: true
            }]
        );
        assert_eq!(
            mapping.keyboard(code, false),
            vec![HostInputEvent::Key {
                code,
                pressed: false
            }]
        );
    }
    configuration.frontend.release = "F12".into();
    assert!(catalog
        .validate(&configuration)
        .unwrap_err()
        .contains("reserved"));
    configuration.frontend.release = "F4".into();
    assert!(catalog.validate(&configuration).is_ok());
    mapping.configure(&configuration, &catalog);
    assert!(mapping.keyboard(0x53, true).is_empty());
}
