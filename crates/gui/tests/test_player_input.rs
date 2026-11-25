#![allow(clippy::unwrap_used)]
mod common;
use debugger::launch::{DeviceKind, MachineSetup};
use gui::host_input::{HostInputEvent, InputConsumer};
use gui::input_mapping::{GamepadInfo, InputMapping};
use gui::player_config::{ConfigurationCatalog, InputSource, PortBinding};

fn two_joysticks() -> (ConfigurationCatalog, gui::player_config::NamedConfiguration) {
    let catalog = ConfigurationCatalog::default();
    let mut config = catalog.selected().unwrap().clone();
    config.controls.ports = [
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

#[test]
fn full_catalog_validation_rejects_broken_identities_and_unused_mapping_fields() {
    assert!(ConfigurationCatalog::default().validate_all().is_ok());
    let rejected = |edit: fn(&mut ConfigurationCatalog), expected: &str| {
        let mut catalog = ConfigurationCatalog::default();
        let mut unused = catalog.mappings[0].clone();
        unused.id = "unused".into();
        catalog.mappings.push(unused);
        edit(&mut catalog);
        assert!(catalog.validate_all().unwrap_err().contains(expected));
    };
    rejected(
        |c| c.selected = "missing".into(),
        "Select an existing configuration",
    );
    rejected(
        |c| c.configurations[0].controls.host.mouse_sensitivity = 0.05,
        "Mouse sensitivity must be between 0.1 and 4",
    );
    rejected(
        |c| {
            c.selected.clear();
            c.configurations[0].id.clear();
        },
        "Configuration identities",
    );
    rejected(
        |c| c.configurations.push(c.configurations[0].clone()),
        "Configuration identities",
    );
    rejected(
        |c| c.mappings[2].id.clear(),
        "Mappings require unique identities",
    );
    rejected(
        |c| c.mappings[2].id = "arrows".into(),
        "Mappings require unique identities",
    );
    rejected(
        |c| c.mappings[2].name.clear(),
        "Mappings require unique identities",
    );
    rejected(|c| c.mappings[2].keys[0] = 0x80, "supported mapping keys");
    rejected(|c| c.mappings[2].buttons[0] = 16, "supported mapping keys");
    rejected(|c| c.mappings[2].axes[0] = 16, "supported mapping keys");
    rejected(|c| c.mappings[2].threshold = 0.05, "supported mapping keys");
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
    mapping.configure(&config.controls, &catalog.mappings);
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
    let mut events = mapping.keyboard(0x31.into(), true); // Z is keyboard Fire 1.
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
    let (catalog, mut config) = two_joysticks();
    config.controls.ports[1].source = InputSource::Keyboard("wasd".into());
    assert!(catalog.validate(&config).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&config.controls, &catalog.mappings);
    assert!(matches!(
        mapping.keyboard(0x4c.into(), true).as_slice(),
        [HostInputEvent::Joystick {
            port: 0,
            up: true,
            ..
        }]
    ));
    assert!(matches!(
        mapping.keyboard(0x11.into(), true).as_slice(),
        [HostInputEvent::Joystick {
            port: 1,
            up: true,
            ..
        }]
    ));
    assert_eq!(
        mapping.keyboard(0x10.into(), true),
        vec![HostInputEvent::Key {
            code: 0x10,
            pressed: true
        }]
    );
    let mut router = gui::keyboard_input::KeyboardRouter::default();
    assert_eq!(
        router.route(
            &common::keyboard::key("F5", true),
            &common::keyboard::policy()
        ),
        gui::keyboard_input::KeyRoute::Command {
            action: gui::keyboard_input::KeyCommand::Pause,
            pressed: true
        },
        "F5 belongs to frontend"
    );
    config.controls.ports[1].source = InputSource::Keyboard("arrows".into());
    assert!(catalog.validate(&config).is_err());
}
#[test]
fn ambiguous_controller_ids_never_drive_a_port() {
    let (catalog, config) = two_joysticks();
    let mut mapping = InputMapping::default();
    mapping.configure(&config.controls, &catalog.mappings);
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
    configuration.controls.host.frontend.pause = "A".into();
    let mut router = gui::keyboard_input::KeyboardRouter::default();
    let mut policy = common::keyboard::policy();
    policy.bindings = configuration.controls.host.frontend;
    for pressed in [true, false] {
        assert_eq!(
            router.route(&common::keyboard::key("KeyA", pressed), &policy),
            gui::keyboard_input::KeyRoute::Command {
                action: gui::keyboard_input::KeyCommand::Pause,
                pressed
            }
        );
    }
}

#[test]
fn two_selected_gamepads_keep_player_states_and_frontend_edges_independent() {
    use gui::player_config::{FrontendAction, GamepadShortcut};
    let (catalog, mut configuration) = two_joysticks();
    configuration.controls.ports[0].source = InputSource::Gamepad {
        identity: "player-one".into(),
        mapping: "arrows".into(),
    };
    configuration
        .controls
        .host
        .gamepad_shortcuts
        .push(GamepadShortcut {
            identity: "player-two".into(),
            button: 9,
            action: FrontendAction::Pause,
        });
    assert!(catalog.validate(&configuration).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration.controls, &catalog.mappings);
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
    configuration.controls.ports.swap(0, 1);
    configuration.controls.host.mouse_sensitivity = 1.5;
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration.controls, &catalog.mappings);
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
    configuration.controls.ports[1].device = DeviceKind::Disconnected;
    configuration.controls.ports[1].source = InputSource::None;
    mapping.configure(&configuration.controls, &catalog.mappings);
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
    configuration.controls.host.keyboard = vec![(0x20.into(), 0x63), (0x21.into(), 0x63)];
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration.controls, &catalog.mappings);
    for (first, second) in [(0x20, 0x21), (0x21, 0x20)] {
        assert_eq!(
            mapping.keyboard(0x20.into(), true),
            vec![HostInputEvent::Key {
                code: 0x63,
                pressed: true
            }]
        );
        assert!(
            mapping.keyboard(0x20.into(), true).is_empty(),
            "Repeat replayed the press"
        );
        assert!(
            mapping.keyboard(0x21.into(), true).is_empty(),
            "Alias replayed the press"
        );
        assert!(
            mapping.keyboard(first.into(), false).is_empty(),
            "Alias released the guest key early"
        );
        assert_eq!(
            mapping.keyboard(second.into(), false),
            vec![HostInputEvent::Key {
                code: 0x63,
                pressed: false
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
    rejected(|_, c| c.launch.drives[0] = false, "DF0");
    rejected(|_, c| c.name.clear(), "configuration name");
    rejected(
        |_, c| c.controls.ports[1] = c.controls.ports[0].clone(),
        "one system mouse",
    );
    rejected(
        |_, c| c.controls.ports[0].source = InputSource::None,
        "compatible",
    );
    rejected(
        |_, c| c.controls.ports[0] = c.controls.ports[1].clone(),
        "conflicting destinations",
    );
    rejected(
        |_, c| c.controls.ports[1].source = InputSource::Keyboard("missing".into()),
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
        |_, c| c.controls.ports = [pad.clone(), pad.clone()],
        "different available controller",
    );
    rejected(
        |catalog, c| {
            c.controls.ports[1] = pad.clone();
            catalog.mappings[0].threshold = 0.05;
        },
        "threshold",
    );
    rejected(
        |_, c| {
            c.controls.ports[1] = pad.clone();
            c.controls.host.gamepad_shortcuts.push(GamepadShortcut {
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
            c.controls.host.gamepad_shortcuts = vec![s.clone(), s];
        },
        "conflicting frontend/joystick",
    );
    rejected(
        |_, c| c.controls.host.keyboard = vec![(0x20.into(), 0x80)],
        "7-bit",
    );
    rejected(
        |_, c| c.controls.host.keyboard = vec![(0x20.into(), 0x21), (0x20.into(), 0x22)],
        "unique physical",
    );
    rejected(
        |_, c| c.controls.host.keyboard = vec![(0x4c.into(), 0x20)],
        "joystick key",
    );
    rejected(
        |_, c| c.controls.host.frontend.pause = "missing-key".into(),
        "physical frontend key",
    );
    rejected(
        |_, c| c.controls.host.frontend.pause = "F12".into(),
        "reserved",
    );
    rejected(
        |_, c| c.controls.host.frontend.pause = c.controls.host.frontend.save.clone(),
        "different keys",
    );
    rejected(
        |_, c| {
            c.controls.host.frontend.pause = "A".into();
            c.controls.host.keyboard = vec![(0x20.into(), 0x21)];
        },
        "keyboard override",
    );
    for key in ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"] {
        rejected(
            |_, c| c.controls.host.frontend.pause = key.into(),
            "joystick mapping",
        );
    }
    let catalog = ConfigurationCatalog::default();
    let mut valid = catalog.selected().unwrap().clone();
    valid.controls.ports[1] = pad;
    valid.controls.host.gamepad_shortcuts.push(GamepadShortcut {
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
    assert!(configuration.controls.host.frontend.release.is_empty());
    assert!(catalog.validate(&configuration).is_ok());
    let mut mapping = InputMapping::default();
    mapping.configure(&configuration.controls, &catalog.mappings);
    for code in [0x45, 0x51] {
        assert_eq!(
            mapping.keyboard(code.into(), true),
            vec![HostInputEvent::Key {
                code,
                pressed: true
            }]
        );
        assert_eq!(
            mapping.keyboard(code.into(), false),
            vec![HostInputEvent::Key {
                code,
                pressed: false
            }]
        );
    }
    configuration.controls.host.frontend.release = "F12".into();
    assert!(catalog
        .validate(&configuration)
        .unwrap_err()
        .contains("reserved"));
    configuration.controls.host.frontend.release = "F4".into();
    assert!(catalog.validate(&configuration).is_ok());
    mapping.configure(&configuration.controls, &catalog.mappings);
    let mut policy = common::keyboard::policy();
    policy.bindings = configuration.controls.host.frontend;
    let mut router = gui::keyboard_input::KeyboardRouter::default();
    assert_eq!(
        router.route(&common::keyboard::key("F4", true), &policy),
        gui::keyboard_input::KeyRoute::Command {
            action: gui::keyboard_input::KeyCommand::Release,
            pressed: true
        }
    );
}

#[test]
fn both_default_keyboard_presets_send_diagonals_and_independent_fire_contacts() {
    let catalog = ConfigurationCatalog::default();
    assert_eq!(catalog.mappings.len(), 2);
    for (id, name, keys) in [
        ("arrows", "Arrow keys", [0x4c, 0x4d, 0x4f, 0x4e, 0x31, 0x32]),
        ("wasd", "WASD", [0x11, 0x21, 0x20, 0x22, 0x26, 0x27]),
    ] {
        let binding = catalog.mapping(id).unwrap();
        assert_eq!((&*binding.name, binding.keys), (name, keys));
        let mut controls = catalog.selected().unwrap().controls.clone();
        controls.ports[1].source = InputSource::Keyboard(id.into());
        let mut input = InputMapping::default();
        input.configure(&controls, &catalog.mappings);
        let [up, _, _, right, fire1, fire2] = keys;
        for key in [up, right, fire1] {
            assert!(matches!(
                input.keyboard(key.into(), true).as_slice(),
                [HostInputEvent::Joystick { port: 1, .. }]
            ));
        }
        assert_eq!(
            input.keyboard(fire2.into(), true),
            vec![HostInputEvent::Joystick {
                port: 1,
                up: true,
                down: false,
                left: false,
                right: true,
                fire1: true,
                fire2: true,
            }]
        );
        assert_eq!(
            input.keyboard(fire1.into(), false),
            vec![HostInputEvent::Joystick {
                port: 1,
                up: true,
                down: false,
                left: false,
                right: true,
                fire1: false,
                fire2: true,
            }]
        );
        input.release();
        assert!(input.keyboard(fire2.into(), false).is_empty());
        for key in [0x0f, 0x3c] {
            assert_eq!(
                input.keyboard(key.into(), true),
                vec![HostInputEvent::Key {
                    code: key,
                    pressed: true
                }]
            );
        }
    }
}

#[test]
fn joystick_position_is_not_a_held_alias_of_its_guest_destination() {
    let catalog = ConfigurationCatalog::default();
    let mut controls = catalog.selected().unwrap().controls.clone();
    controls.host.keyboard.push((0x34.into(), 0x4c));
    let mut input = InputMapping::default();
    input.configure(&controls, &catalog.mappings);
    assert!(matches!(
        input.keyboard(0x4c.into(), true).as_slice(),
        [HostInputEvent::Joystick { up: true, .. }]
    ));
    for pressed in [true, false] {
        assert_eq!(
            input.keyboard(0x34.into(), pressed),
            vec![HostInputEvent::Key {
                code: 0x4c,
                pressed
            }]
        );
    }
}

#[test]
fn changing_the_joystick_mapping_frees_every_physical_arrow_for_the_guest() {
    let catalog = ConfigurationCatalog::default();
    let mut controls = catalog.selected().unwrap().controls.clone();
    let mut input = InputMapping::default();
    for preset in ["arrows", "wasd"] {
        controls.ports[1].source = InputSource::Keyboard(preset.into());
        assert!(input.configure(&controls, &catalog.mappings));
        for (index, (position, code)) in [
            ("ArrowUp", 0x4c),
            ("ArrowDown", 0x4d),
            ("ArrowLeft", 0x4f),
            ("ArrowRight", 0x4e),
        ]
        .into_iter()
        .enumerate()
        {
            for pressed in [true, false] {
                let events = input.keyboard(position.to_owned().into(), pressed);
                let expected = if preset == "arrows" {
                    HostInputEvent::Joystick {
                        port: 1,
                        up: pressed && index == 0,
                        down: pressed && index == 1,
                        left: pressed && index == 2,
                        right: pressed && index == 3,
                        fire1: false,
                        fire2: false,
                    }
                } else {
                    HostInputEvent::Key { code, pressed }
                };
                assert_eq!(events, [expected], "{preset} {position}");
            }
        }
    }
    assert_eq!(
        catalog.mapping("arrows").unwrap().keys,
        [0x4c, 0x4d, 0x4f, 0x4e, 0x31, 0x32]
    );
}
