#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use debugger::launch::DeviceKind;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::player_config::{InputSource, NamedConfiguration, PortBinding};
use gui::{EmulatorApp, ViewMode};

fn editor() -> Harness<'static, EmulatorApp> {
    let mut session = Box::new(debugger::DebuggerSession::new());
    session.load_binary(0xc00000, &[0x60, 0xfe], true).unwrap();
    editor_with_session(session)
}

fn editor_with_session(session: Box<debugger::DebuggerSession>) -> Harness<'static, EmulatorApp> {
    let mut app = EmulatorApp::with_session(session);
    let library = tempfile::tempdir().unwrap();
    app.player.library_root = Some(library.path().to_path_buf());
    app.player.activate_selected_configuration();
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(480.0, 640.0))
        .build_ui_state(
            move |ui, app: &mut EmulatorApp| {
                let _library = &library;
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
            app,
        );
    common::settle_harness(&mut h);
    h
}

#[test]
fn cancel_discards_new_profile_shared_mapping_and_invalid_edits() {
    let mut h = editor();
    let catalog = h.state().player.catalog.clone();
    let machine = common::snapshot(h.state_mut());
    h.get_by_label("Create configuration").click_accesskit();
    common::settle_harness(&mut h);
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    draft.mappings[0].keys[0] = 0x11;
    draft.mappings.push(gui::player_config::JoystickMapping {
        id: "created".into(),
        ..Default::default()
    });
    draft.configurations[0].name.clear();
    h.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.catalog, catalog);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    assert_eq!(common::snapshot(h.state_mut()), machine);
}

#[test]
fn save_changes_active_identity_controls_without_applying_future_launch_choices() {
    let mut h = editor();
    let mut expected = common::snapshot(h.state_mut());
    let catalog = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    catalog.configurations[0].controls.ports = [
        PortBinding {
            device: DeviceKind::Joystick,
            source: InputSource::Keyboard("arrows".into()),
        },
        PortBinding {
            device: DeviceKind::Mouse,
            source: InputSource::SystemMouse,
        },
    ];
    catalog.configurations[0].controls.host.keyboard = vec![(0x20.into(), 0x21)];
    catalog.configurations[0].launch.preset = config::A500Preset::ExpandedPowerUser;
    catalog.configurations[0].launch.rom = "future.rom".into();
    catalog.configurations[0].launch.drives[1] = true;
    catalog.mappings[0].keys[0] = 0x11;
    catalog.configurations.push(NamedConfiguration {
        id: "future".into(),
        ..Default::default()
    });
    catalog.selected = "future".into();
    let committed = catalog.clone();
    let generation = h.state().host.status.generation;
    h.get_by_label("Save").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(h.state().host.status.generation, generation + 1);
    assert!(!h.state().host.status.running);
    let active = h.state().player.active_configuration.as_ref().unwrap();
    assert_eq!(active.id, "standard");
    assert_eq!(active.controls, committed.configurations[0].controls);
    assert_eq!(active.launch.preset, config::A500Preset::Standard1Mb);
    let after = common::snapshot(h.state_mut());
    expected.game_ports.state.port1 = after.game_ports.state.port1;
    expected.game_ports.state.port2 = after.game_ports.state.port2;
    assert_eq!(after, expected);
    assert_eq!(
        h.state().host.status.ports,
        [DeviceKind::Joystick, DeviceKind::Mouse]
    );
}

#[test]
fn invalid_profile_cannot_publish_shared_mapping_or_controls() {
    let mut h = editor();
    let committed = h.state().player.catalog.clone();
    let machine = common::snapshot(h.state_mut());
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    draft.configurations.push(NamedConfiguration {
        id: "other".into(),
        ..Default::default()
    });
    draft.configurations[1].name.clear();
    draft.mappings[0].keys[0] = 0x11;
    h.get_by_label("Save").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_some());
    h.get_by_label("Enter a configuration name");
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(common::snapshot(h.state_mut()), machine);
    h.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

#[test]
fn invalid_reusable_mapping_cannot_publish_even_when_no_profile_uses_it() {
    for referenced in [true, false] {
        let mut h = editor();
        let committed = h.state().player.catalog.clone();
        let controls = h.state().player.active_configuration.clone();
        let generation = h.state().host.status.generation;
        let machine = common::snapshot(h.state_mut());
        let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
        draft.configurations[0].name = "Edited profile".into();
        let index = if referenced {
            0
        } else {
            let mut mapping = draft.mappings[0].clone();
            mapping.id = "unused".into();
            draft.mappings.push(mapping);
            1
        };
        draft.mappings[index].name.clear();
        h.get_by_label("Save").click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_label("Mappings require unique identities and a name");
        assert!(h.state().player.editor.is_some());
        assert_eq!(h.state().player.catalog, committed);
        assert_eq!(h.state().player.active_configuration, controls);
        assert_eq!(h.state().host.status.generation, generation);
        assert_eq!(common::snapshot(h.state_mut()), machine);
        h.get_by_label("Cancel").click_accesskit();
        common::settle_harness(&mut h);
        assert!(h.state().player.editor.is_none());
        assert_eq!(h.state().player.screen, PlayerScreen::Session);
        assert_eq!(h.state().player.catalog, committed);
        assert_eq!(common::snapshot(h.state_mut()), machine);
    }
}

fn unavailable_controller_navigation(discard: bool) {
    let mut h = editor();
    h.state_mut().cancel_configuration_edit();
    let source = InputSource::Gamepad {
        identity: "lost-controller".into(),
        mapping: "arrows".into(),
    };
    h.state_mut()
        .player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .ports[1]
        .source = source.clone();
    h.state_mut().player.catalog.configurations[0]
        .controls
        .ports[1]
        .source = source;
    h.state_mut().navigate_player(PlayerScreen::Configuration);
    common::settle_harness(&mut h);
    let generation = h.state().host.status.generation;
    let machine = common::snapshot(h.state_mut());
    if discard {
        h.state_mut()
            .player
            .editor
            .as_mut()
            .unwrap()
            .catalog
            .configurations[0]
            .name = "Dirty".into();
        h.get_by_label("Return to session").click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_label("Discard").click_accesskit();
    } else {
        h.get_by_label("Debug").click_accesskit();
    }
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    assert!(h
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("unavailable"));
    assert!(h.state().player.editor.is_some());
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), machine);
    h.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

#[test]
fn debug_rejects_unavailable_session_controller() {
    unavailable_controller_navigation(false);
}

#[test]
fn discard_to_session_rejects_unavailable_controller() {
    unavailable_controller_navigation(true);
}

#[test]
fn dirty_navigation_offers_keep_discard_and_save_at_narrow_size() {
    for action in ["Keep editing", "Discard", "Save"] {
        let mut h = editor();
        h.state_mut()
            .player
            .editor
            .as_mut()
            .unwrap()
            .catalog
            .configurations[0]
            .name = "Edited".into();
        h.get_by_label("Manual setup").click_accesskit();
        common::settle_harness(&mut h);
        assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
        h.get_by_label("Unsaved configuration changes");
        // The last Save belongs to the modal; the footer remains behind it.
        h.get_all_by_label(action)
            .filter(|node| node.accesskit_node().role() == egui::accesskit::Role::Button)
            .last()
            .unwrap()
            .click_accesskit();
        common::settle_harness(&mut h);
        assert_eq!(
            h.state().player.screen,
            if action == "Keep editing" {
                PlayerScreen::Configuration
            } else {
                PlayerScreen::Setup
            }
        );
        assert_eq!(
            h.state().player.catalog.configurations[0].name,
            if action == "Save" {
                "Edited"
            } else {
                "Standard A500"
            }
        );
        assert_eq!(h.state().player.editor.is_some(), action == "Keep editing");
    }
}

#[test]
fn failed_owner_request_retains_draft_catalog_and_session_controls() {
    let mut h = editor();
    let committed = h.state().player.catalog.clone();
    let generation = h.state().host.status.generation;
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .controls
        .ports[1] = PortBinding {
        device: DeviceKind::Disconnected,
        source: InputSource::None,
    };
    h.state_mut().save_configuration_edit();
    // A competing owner request demonstrates rejection at the real stopped guard.
    h.state_mut()
        .host
        .runtime
        .as_mut()
        .unwrap()
        .submit(HostCommand::Run)
        .unwrap();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(
        h.state().host.status.ports,
        [DeviceKind::Mouse, DeviceKind::Joystick]
    );
    assert!(h.state().player.editor.is_some());
    common::command(h.state_mut(), HostCommand::Pause);
    common::wait_harness(&mut h, |app| !app.host.status.running);
    h.state_mut().cancel_configuration_edit();
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

#[test]
fn developer_navigation_cannot_bypass_dirty_editor() {
    let mut h = editor();
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .name = "Dirty".into();
    h.state_mut().set_view_mode(ViewMode::Developer);
    common::settle_harness(&mut h);
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    h.get_by_label("Discard").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

#[test]
fn return_still_validates_host_sources_and_clean_debug_navigation_leaves_editor() {
    let mut h = editor();
    h.get_by_label("Debug").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    h.state_mut()
        .player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .ports[1]
        .source = InputSource::Gamepad {
        identity: "missing".into(),
        mapping: "arrows".into(),
    };
    h.state_mut().navigate_player(PlayerScreen::Setup);
    h.state_mut().navigate_player(PlayerScreen::Session);
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
    assert!(h
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("unavailable"));
    assert!(h.state().player.editor.is_some());
}

#[test]
fn pending_save_blocks_cancel_navigation_and_duplicate_save() {
    let mut h = editor();
    let committed = h.state().player.catalog.clone();
    let generation = h.state().host.status.generation;
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    draft.configurations[0].name = "Saved draft".into();
    draft.configurations[0].controls.ports[1] = PortBinding {
        device: DeviceKind::Disconnected,
        source: InputSource::None,
    };
    let expected = draft.clone();
    h.state_mut().save_configuration_edit();
    // Commands are still GUI-owned here: exercise the pending transaction
    // before pumping any owner replies, independently of thread scheduling.
    h.state_mut().cancel_configuration_edit();
    h.state_mut().navigate_player(PlayerScreen::Setup);
    h.state_mut().set_view_mode(ViewMode::Developer);
    h.state_mut().save_configuration_edit();
    assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(h.state().player.editor.as_ref().unwrap().catalog, expected);
    assert_eq!(h.state().host.status.generation, generation);
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.catalog, expected);
    assert_eq!(h.state().host.status.generation, generation + 1);
    assert_eq!(h.state().host.status.ports[1], DeviceKind::Disconnected);
}

#[test]
fn busy_or_stopped_owner_cannot_publish_configuration_draft() {
    for shutdown in [false, true] {
        let mut h = editor();
        let committed = h.state().player.catalog.clone();
        let controls = h.state().player.active_configuration.clone();
        let (cck, machine) = if shutdown {
            let reply = common::idle_command(h.state_mut(), HostCommand::ReadCpu);
            assert!(reply.value.is_ok());
            (reply.status.cck, None)
        } else {
            let state = common::snapshot(h.state_mut());
            (state.cck, Some(state))
        };
        let generation = h.state().host.status.generation;
        h.state_mut()
            .player
            .editor
            .as_mut()
            .unwrap()
            .catalog
            .configurations[0]
            .name = "Retained draft".into();
        if shutdown {
            assert!(common::command(h.state_mut(), HostCommand::Shutdown)
                .value
                .is_ok());
        } else {
            h.state_mut().host.queue(HostCommand::ReadCpu);
        }
        h.state_mut().save_configuration_edit();
        assert_eq!(h.state().player.catalog, committed);
        assert_eq!(h.state().player.active_configuration, controls);
        assert_eq!(h.state().host.status.generation, generation);
        assert_eq!(h.state().host.status.cck, cck);
        assert_eq!(
            h.state()
                .player
                .editor
                .as_ref()
                .unwrap()
                .catalog
                .configurations[0]
                .name,
            "Retained draft"
        );
        h.step();
        h.get_by_label(if shutdown {
            "The machine owner is unavailable"
        } else {
            "Wait for the current machine to pause before saving"
        });
        if let Some(machine) = machine {
            assert_eq!(common::snapshot(h.state_mut()), machine);
        }
        h.state_mut().cancel_configuration_edit();
        assert!(h.state().player.editor.is_none());
        assert_eq!(h.state().player.catalog, committed);
    }
}

#[test]
fn owner_failure_during_save_retains_draft_and_committed_controls() {
    fn invariant_read(
        memory: &mut physical_memory::PhysicalMemory,
        address: u32,
    ) -> physical_memory::BusResult<u16> {
        use config::chip_ram::ChipRamAccess;
        memory.read_chip_word(address);
        physical_memory::BusResult::Ready(memory.read_chip_word(address))
    }
    let mut session = Box::new(debugger::DebuggerSession::new());
    session
        .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
        .unwrap();
    // Reuse the runtime failure fixture's bank dispatch injection. No guest
    // execution occurs until the explicit command below; the failed owner
    // is never resumed or queried for a partially advanced machine.
    session.machine.physical_memory.bank_map[0].read_word = invariant_read;
    let mut h = editor_with_session(session);
    let committed = h.state().player.catalog.clone();
    let controls = h.state().player.active_configuration.clone();
    let generation = h.state().host.status.generation;
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    draft.configurations[0].controls.ports[1] = PortBinding {
        device: DeviceKind::Disconnected,
        source: InputSource::None,
    };
    let expected = draft.clone();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.state().host.is_idle() {
        h.state_mut().save_configuration_edit();
        assert!(
            std::time::Instant::now() < deadline,
            "Save did not queue owner work"
        );
        std::thread::yield_now();
    }
    h.state_mut()
        .host
        .runtime
        .as_mut()
        .unwrap()
        .submit(HostCommand::StepInstruction)
        .unwrap();
    // Establish terminal failure before the GUI can submit its configuration
    // command. A normal command rejection must not satisfy this failure case.
    loop {
        match h.state_mut().host.runtime.as_mut().unwrap().poll() {
            Err(gui::host_runtime::HostRuntimeError::Fatal(message)) => {
                assert!(message.starts_with("Chip RAM access invariant:"));
                break;
            }
            Err(error) => panic!("Unexpected owner failure: {error:?}"),
            Ok(_) => {}
        }
        assert!(std::time::Instant::now() < deadline, "Owner did not fail");
        std::thread::yield_now();
    }
    common::settle_harness(&mut h);
    h.get_by_label("The machine owner is unavailable");
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(h.state().player.active_configuration, controls);
    assert_eq!(h.state().player.editor.as_ref().unwrap().catalog, expected);
    assert_eq!(h.state().host.status.generation, generation);
    assert!(!h.state().host.status.running);
    h.state_mut().save_configuration_edit();
    assert_eq!(h.state().player.catalog, committed);
    h.state_mut().cancel_configuration_edit();
    assert!(h.state().player.editor.is_none());
}

#[test]
fn switching_profiles_in_dropdown_preserves_both_drafts_until_save_or_cancel() {
    for save in [false, true] {
        let mut h = editor();
        let committed = h.state().player.catalog.clone();
        let machine = common::snapshot(h.state_mut());
        h.state_mut()
            .player
            .editor
            .as_mut()
            .unwrap()
            .catalog
            .configurations[0]
            .name = "Edited first".into();
        h.get_by_label("Create configuration").click_accesskit();
        common::settle_harness(&mut h);
        let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
        draft.configurations[1].name = "Edited second".into();
        draft.mappings[0].keys[0] = 0x11;
        common::settle_harness(&mut h);
        for (name, expected_id) in [("Edited first", Some("standard")), ("Edited second", None)] {
            // AccessKit selection does not emit a pointer click to dismiss the
            // popup. Use physical pointer events for opening and selecting.
            for option in [None, Some(name)] {
                let bounds = if let Some(name) = option {
                    h.get_by_label(name)
                        .accesskit_node()
                        .bounding_box()
                        .unwrap()
                } else {
                    h.get_all_by_role(egui::accesskit::Role::ComboBox)
                        .next()
                        .unwrap()
                        .accesskit_node()
                        .bounding_box()
                        .unwrap()
                };
                let position = egui::pos2(
                    ((bounds.x0 + bounds.x1) / 2.0) as f32,
                    ((bounds.y0 + bounds.y1) / 2.0) as f32,
                );
                h.input_mut()
                    .events
                    .push(egui::Event::PointerMoved(position));
                h.step();
                for pressed in [true, false] {
                    h.input_mut().events.push(egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                    h.step();
                }
                common::settle_harness(&mut h);
            }
            if let Some(id) = expected_id {
                assert_eq!(
                    h.state().player.editor.as_ref().unwrap().catalog.selected,
                    id
                );
            }
        }
        let expected = h.state().player.editor.as_ref().unwrap().catalog.clone();
        assert_eq!(expected.configurations[0].name, "Edited first");
        assert_eq!(expected.configurations[1].name, "Edited second");
        assert_eq!(expected.mappings[0].keys[0], 0x11);
        assert_eq!(h.state().player.catalog, committed);
        h.get_by_label(if save { "Save" } else { "Cancel" })
            .click_accesskit();
        common::settle_harness(&mut h);
        assert!(h.state().player.editor.is_none());
        assert_eq!(
            h.state().player.catalog,
            if save { expected } else { committed }
        );
        assert_eq!(common::snapshot(h.state_mut()), machine);
    }
}

#[test]
fn quick_load_rejects_unavailable_current_controller_without_mutation() {
    let mut h = editor();
    h.state_mut().cancel_configuration_edit();
    common::command(h.state_mut(), HostCommand::SaveQuickSlot(1));
    common::command(
        h.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: 0x12345678,
        },
    );
    let machine = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    h.state_mut()
        .player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .ports[1]
        .source = InputSource::Gamepad {
        identity: "missing".into(),
        mapping: "arrows".into(),
    };
    common::settle_harness(&mut h);
    // Exercise the real frontend shortcut, rather than the owner rejection.
    h.key_press(egui::Key::F9);
    common::settle_harness(&mut h);
    assert!(h
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("unavailable"));
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), machine);
    assert!(h.state().host.status.quick_slots[0]);
}

#[test]
fn input_contention_defers_draft_publication_until_sender_recovers() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::Duration;
    // The owner repaint context is separate from the GUI frame context, so
    // holding the mailbox in its callback cannot lock the GUI context itself.
    let owner_context = egui::Context::default();
    let mut app = EmulatorApp::with_session(Box::new(debugger::DebuggerSession::new()));
    let library = tempfile::tempdir().unwrap();
    app.player.library_root = Some(library.path().to_path_buf());
    app.player.activate_selected_configuration();
    app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut app, &owner_context);
    let mut h = Harness::builder()
        .with_size(egui::vec2(480.0, 640.0))
        .build_ui_state(
            |ui, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
            app,
        );
    common::settle_harness(&mut h);
    let committed = h.state().player.catalog.clone();
    let controls = h.state().player.active_configuration.clone();
    let machine = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .name = "Deferred draft".into();
    let expected = h.state().player.editor.as_ref().unwrap().catalog.clone();
    h.state_mut().save_configuration_edit(); // Name-only save is ready to publish.
                                             // Keep video consumption behind a real generation barrier while the GUI
                                             // tests input contention; its independent repaint retry uses the owner context.
    let sender = h.state().host.runtime.as_ref().unwrap().input_sender();
    h.state_mut()
        .host
        .runtime
        .as_mut()
        .unwrap()
        .submit(HostCommand::ReconfigurePorts([
            DeviceKind::Mouse,
            DeviceKind::Joystick,
        ]))
        .unwrap();
    let sample = |identity: &str| gui::input_mapping::GamepadSample {
        identity: identity.into(),
        name: "Contention controller".into(),
        axes: vec![0.0; 2],
        buttons: vec![false; 16],
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !sender
        .gamepads()
        .is_some_and(|pads| pads.iter().any(|p| p.identity == "ready-pad"))
    {
        sender.try_gamepads(vec![sample("ready-pad")], generation + 1, true);
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    for _ in 0..2 {
        owner_context
            .run_pass(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
    }
    let (entered_tx, entered_rx) = mpsc::channel();
    static RELEASE: AtomicBool = AtomicBool::new(false);
    RELEASE.store(false, Ordering::SeqCst);
    let armed = AtomicBool::new(true);
    owner_context.set_request_repaint_callback(move |_| {
        if armed.swap(false, Ordering::SeqCst) {
            entered_tx.send(()).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while !RELEASE.load(Ordering::SeqCst) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "GUI did not release input producer"
                );
                std::thread::yield_now();
            }
        }
    });
    let sender = h.state().host.runtime.as_ref().unwrap().input_sender();
    let producer = std::thread::spawn(move || {
        assert!(sender.try_gamepads(
            vec![gui::input_mapping::GamepadSample {
                identity: "contention-pad".into(),
                name: "Contention controller".into(),
                axes: vec![0.0; 2],
                buttons: vec![false; 16],
            }],
            generation + 1,
            true
        ));
    });
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    h.step();
    let retained = h.state().player.editor.as_ref().map(|e| e.catalog.clone());
    let unpublished = h.state().player.catalog.clone();
    RELEASE.store(true, Ordering::SeqCst);
    producer.join().unwrap();
    assert_eq!(retained, Some(expected.clone()));
    assert_eq!(unpublished, committed);
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.catalog, expected);
    assert_eq!(h.state().player.active_configuration, controls);
    assert_eq!(h.state().host.status.generation, generation + 1);
    assert_eq!(common::snapshot(h.state_mut()), machine);
}

fn click_binding_control(h: &mut Harness<'_, EmulatorApp>, label: &str) {
    h.get_by_label(label).click_accesskit();
    common::settle_harness(h);
}

#[track_caller]
fn combo_box_showing<'a>(
    h: &'a Harness<'_, EmulatorApp>,
    selected: &str,
) -> egui_kittest::Node<'a> {
    // Check that the selected text is painted inside both the widget and the clip rectangle.
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .filter(|node| {
            let bounds = node.rect();
            h.output().shapes.iter().any(|shape| {
                matches!(
                    &shape.shape,
                    egui::Shape::Text(text)
                        if text.galley.text() == selected
                            && !text.galley.elided
                            && bounds.contains(text.pos)
                            && shape.clip_rect.contains(text.pos)
                )
            })
        })
        .last()
        .unwrap_or_else(|| panic!("No visible combo box showing {selected:?}"))
}

fn new_key_override_showing<'a>(
    h: &'a Harness<'_, EmulatorApp>,
    selected: &str,
) -> egui_kittest::Node<'a> {
    use egui::accesskit::Role;

    let row = h
        .get_by_role_and_label(Role::Button, "Add key override")
        .parent()
        .unwrap();
    let combo = row.get_by_role(Role::ComboBox);
    assert_eq!(combo.value().as_deref(), Some(selected));
    combo
}

fn physical_edge(
    h: &mut Harness<'_, EmulatorApp>,
    position: &str,
    pressed: bool,
    repeat: bool,
) -> bool {
    let mut key = common::keyboard::key(position, pressed);
    key.repeat = repeat;
    gui::keyboard_input::receive_key(&h.ctx, key, Some(h.state().host.status.generation))
}

fn record_key(h: &mut Harness<'_, EmulatorApp>, position: &str) {
    assert!(physical_edge(h, position, true, false));
    common::settle_harness(h);
    assert!(physical_edge(h, position, false, false));
    common::settle_harness(h);
}

fn wait_guest_key(h: &mut Harness<'_, EmulatorApp>, code: u8, held: bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        common::settle_harness(h);
        if common::observe_input(h.state_mut(), Some(code)).key_held == Some(held) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Guest key {code:#x} did not become held={held}"
        );
    }
}

#[test]
fn keyboard_binding_buttons_record_fresh_edges_and_keep_draft_isolated() {
    let mut h = editor();
    h.set_size(egui::vec2(360.0, 790.0));
    common::settle_harness(&mut h);
    for label in [
        "Emulator shortcuts",
        "Open developer view: F12",
        "Release mouse: middle mouse button. You can also assign a key.",
        "Pause / Resume",
        "Quick save",
        "Quick load",
        "Unassigned",
    ] {
        let bounds = h.get_by_label(label).rect();
        assert!(
            bounds.left() >= 0.0 && bounds.right() <= 360.0,
            "{label}: {bounds:?}"
        );
    }
    let before = h.state().player.catalog.clone();
    click_binding_control(&mut h, "F5");
    h.get_by_label("Press a key");
    record_key(&mut h, "F12");
    h.get_by_label_contains("F12 is reserved");
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    record_key(&mut h, "F6");
    h.get_by_label_contains("different keys");
    assert!(physical_edge(&mut h, "F7", true, true));
    common::settle_harness(&mut h);
    h.get_by_label("Press a key");
    click_binding_control(&mut h, "Cancel recording");
    assert_eq!(h.state().player.editor.as_ref().unwrap().catalog, before);

    click_binding_control(&mut h, "F5");
    record_key(&mut h, "F7");
    h.get_by_label("F7");
    assert_eq!(h.state().player.catalog, before);
    click_binding_control(&mut h, "Unassigned");
    record_key(&mut h, "Escape");
    h.get_by_label_contains("Choose another mouse release key");
    click_binding_control(&mut h, "Cancel recording");
    click_binding_control(&mut h, "Unassigned");
    record_key(&mut h, "F2");
    click_binding_control(&mut h, "Unassign");
    h.get_by_label("Unassigned");

    h.set_size(egui::vec2(1280.0, 900.0));
    common::settle_harness(&mut h);
    click_binding_control(&mut h, "Amiga keyboard remapping");
    new_key_override_showing(&h, "Escape");
    click_binding_control(&mut h, "Add key override");
    h.get_by_label("Host key for Amiga Escape");
    record_key(&mut h, "ArrowUp");
    h.get_by_label_contains("Arrow keys Up on port 2");
    record_key(&mut h, "KeyV");
    assert_eq!(
        h.state()
            .player
            .editor
            .as_ref()
            .unwrap()
            .catalog
            .configurations[0]
            .controls
            .host
            .keyboard,
        vec![("KeyV".to_owned().into(), 0x45)]
    );

    click_binding_control(&mut h, "Arrow keys");
    for label in ["Up", "Down", "Left", "Right", "Fire 1", "Fire 2"] {
        h.get_by_label(label);
    }
    click_binding_control(&mut h, "Z");
    record_key(&mut h, "Escape");
    let generation = h.state().host.status.generation;
    click_binding_control(&mut h, "Save");
    assert_eq!(h.state().host.status.generation, generation + 1);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.catalog.mappings[0].keys[4], 0x45);
    assert_eq!(
        h.state()
            .player
            .active_configuration
            .as_ref()
            .unwrap()
            .controls
            .host
            .frontend
            .pause,
        "F7"
    );
    assert_eq!(
        h.state()
            .player
            .catalog
            .selected()
            .unwrap()
            .controls
            .host
            .keyboard,
        vec![("KeyV".to_owned().into(), 0x45)]
    );

    assert!(physical_edge(&mut h, "F7", true, false));
    assert!(physical_edge(&mut h, "F7", false, false));
    common::wait_harness(&mut h, |app| app.host.status.running);
    assert!(physical_edge(&mut h, "KeyV", true, false));
    wait_guest_key(&mut h, 0x45, true);
    assert_eq!(
        common::observe_input(h.state_mut(), Some(0x34)).key_held,
        Some(false)
    );
    assert!(physical_edge(&mut h, "KeyV", false, false));
    wait_guest_key(&mut h, 0x45, false);
    assert!(physical_edge(&mut h, "F7", true, false));
    assert!(physical_edge(&mut h, "F7", false, false));
    common::wait_harness(&mut h, |app| !app.host.status.running);

    h.state_mut().navigate_player(PlayerScreen::Configuration);
    common::settle_harness(&mut h);
    if h.query_by_label("Remove override").is_none() {
        click_binding_control(&mut h, "Amiga keyboard remapping");
    }
    click_binding_control(&mut h, "Remove override");
    click_binding_control(&mut h, "Save");
    assert!(h
        .state()
        .player
        .catalog
        .selected()
        .unwrap()
        .controls
        .host
        .keyboard
        .is_empty());
}

#[test]
fn keyboard_override_records_menu_without_a_default_guest_key() {
    let mut h = editor();
    h.set_size(egui::vec2(1280.0, 900.0));
    common::settle_harness(&mut h);
    click_binding_control(&mut h, "Amiga keyboard remapping");
    assert_eq!(gui::host_keys::amiga_key_code("ContextMenu"), None);
    for (previous, destination, source) in [
        ("Escape", "Keypad (", "ContextMenu"),
        ("Keypad (", "Keypad )", "BracketRight"),
    ] {
        new_key_override_showing(&h, previous).click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_role_and_label(egui::accesskit::Role::Button, destination)
            .click_accesskit();
        common::settle_harness(&mut h);
        // AccessKit selection leaves the popup open, so toggle it closed.
        new_key_override_showing(&h, destination).click_accesskit();
        common::settle_harness(&mut h);
        click_binding_control(&mut h, "Add key override");
        h.get_by_label(&format!("Host key for Amiga {destination}"));
        record_key(&mut h, source);
        assert!(h.query_by_label("Press a key").is_none());
    }
    h.get_by_label("Menu");
    click_binding_control(&mut h, "Save");

    let root = h.state().player.library_root.as_ref().unwrap();
    let mut reopened_app = EmulatorApp::default();
    reopened_app.navigate_player(PlayerScreen::Configuration);
    let mut reopened = Harness::builder()
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), reopened_app);
    common::settle_harness(&mut reopened);
    common::select_library(&mut reopened, root);
    assert_eq!(
        reopened
            .state()
            .player
            .catalog
            .selected()
            .unwrap()
            .controls
            .host
            .keyboard,
        vec![
            ("ContextMenu".to_owned().into(), 0x5a),
            ("BracketRight".to_owned().into(), 0x5b)
        ]
    );
    assert!(physical_edge(&mut h, "F5", true, false));
    assert!(physical_edge(&mut h, "F5", false, false));
    common::wait_harness(&mut h, |app| app.host.status.running);
    for (source, guest) in [("ContextMenu", 0x5a), ("BracketRight", 0x5b)] {
        assert!(physical_edge(&mut h, source, true, false));
        wait_guest_key(&mut h, guest, true);
        assert_eq!(
            common::observe_input(h.state_mut(), Some(0x1b)).key_held,
            Some(false)
        );
        assert!(physical_edge(&mut h, source, true, true));
        assert!(physical_edge(&mut h, source, false, false));
        wait_guest_key(&mut h, guest, false);
    }
}

#[test]
fn focus_loss_cancels_recording_without_applying_the_queued_press() {
    let mut h = editor();
    click_binding_control(&mut h, "F5");
    assert!(physical_edge(&mut h, "F7", true, false));
    let generation = h.state().host.status.generation;
    gui::keyboard_input::receive_focus(&h.ctx, false, Some(generation));
    h.input_mut().focused = false;
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Press a key").is_none());
    assert_eq!(
        h.state()
            .player
            .editor
            .as_ref()
            .unwrap()
            .catalog
            .selected()
            .unwrap()
            .controls
            .host
            .frontend
            .pause,
        "F5"
    );
    h.input_mut().focused = true;
    common::settle_harness(&mut h);
    click_binding_control(&mut h, "F5");
    record_key(&mut h, "F7");
    h.get_by_label("F7");
}

fn sample_edit_pads(
    h: &mut Harness<'_, EmulatorApp>,
    pads: &[(&str, &str, Option<usize>, [f32; 4])],
) {
    use gui::input_mapping::GamepadSample;
    let generation = h.state().host.status.generation;
    let sender = h.state().host.runtime.as_ref().unwrap().input_sender();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let samples = pads
            .iter()
            .map(|&(identity, name, button, axes)| {
                let mut buttons = vec![false; 16];
                if let Some(button) = button {
                    buttons[button] = true;
                }
                GamepadSample {
                    identity: identity.into(),
                    name: name.into(),
                    axes: axes.to_vec(),
                    buttons,
                }
            })
            .collect();
        if sender.try_gamepads(samples, generation, true) {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    common::settle_harness(h);
}

#[test]
fn controller_editor_preserves_unavailable_bindings_and_records_selected_fresh_inputs() {
    use gui::player_config::{FrontendAction, GamepadShortcut};
    let mut h = editor();
    h.set_size(egui::vec2(960.0, 1080.0));
    let original = GamepadShortcut {
        identity: "selected".into(),
        button: 9,
        action: FrontendAction::Pause,
    };
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .controls
        .host
        .gamepad_shortcuts = vec![original.clone()];
    common::settle_harness(&mut h);
    h.get_by_label("No controller detected");
    click_binding_control(&mut h, "Controller shortcuts");
    assert!(h
        .get_by_label("Add shortcut")
        .accesskit_node()
        .is_disabled());
    h.get_by_label_contains("Button: Start");

    sample_edit_pads(&mut h, &[("other", "Other pad", None, [0.0; 4])]);
    assert!(!combo_box_showing(&h, "Unavailable: selected")
        .accesskit_node()
        .is_disabled());
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    let selected = ("selected", "Main pad", None, [0.0; 4]);
    let other = ("other", "Other pad", None, [0.0; 4]);
    sample_edit_pads(&mut h, &[selected, other]);
    combo_box_showing(&h, "Main pad");
    assert!(!h.get_by_label("Start").accesskit_node().is_disabled());
    for name in ["Controller", "Button", "Action", "Remove shortcut"] {
        assert!(h.query_all_by_label(name).count() > 0);
    }
    sample_edit_pads(
        &mut h,
        &[("selected", "Main pad", Some(9), [0.0; 4]), other],
    );
    click_binding_control(&mut h, "Start");
    sample_edit_pads(
        &mut h,
        &[
            ("selected", "Main pad", Some(9), [0.0; 4]),
            ("other", "Other pad", Some(4), [0.0; 4]),
        ],
    );
    h.get_by_label("Press a controller input");
    sample_edit_pads(&mut h, &[selected, other]);
    sample_edit_pads(
        &mut h,
        &[("selected", "Main pad", Some(4), [0.0; 4]), other],
    );
    h.get_by_label("Left shoulder");
    let shortcuts = &h
        .state()
        .player
        .editor
        .as_ref()
        .unwrap()
        .catalog
        .configurations[0]
        .controls
        .host
        .gamepad_shortcuts;
    assert_eq!(
        shortcuts,
        &[GamepadShortcut {
            button: 4,
            ..original
        }]
    );
    assert!(!h.state().host.status.running);

    click_binding_control(&mut h, "Left shoulder");
    sample_edit_pads(&mut h, &[other]);
    assert!(h.query_by_label("Press a controller input").is_none());
    h.get_by_label_contains("recording ended");
    sample_edit_pads(&mut h, &[selected, other]);
    click_binding_control(&mut h, "Controller shortcuts");
    click_binding_control(&mut h, "Arrow keys");
    combo_box_showing(&h, "Other pad").click();
    common::settle_harness(&mut h);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Main pad")
        .click();
    common::settle_harness(&mut h);
    combo_box_showing(&h, "Main pad");
    click_binding_control(&mut h, "Left stick horizontal");
    h.get_by_label("Controller: Main pad");
    sample_edit_pads(
        &mut h,
        &[("selected", "Main pad", None, [0.0, 0.0, 0.48, 0.0]), other],
    );
    h.get_by_label("Press a controller input");
    sample_edit_pads(
        &mut h,
        &[("selected", "Main pad", None, [0.0, 0.0, -0.8, 0.0]), other],
    );
    h.get_by_label("Right stick horizontal");
    click_binding_control(&mut h, "Save");
    let mapping = h.state().player.catalog.mapping("arrows").unwrap();
    assert_eq!(mapping.axes, [2, 1]);
    assert_eq!(mapping.threshold, 0.5);

    let mut controls = h
        .state()
        .player
        .active_configuration
        .as_ref()
        .unwrap()
        .controls
        .clone();
    controls.ports[1].source = InputSource::Gamepad {
        identity: "selected".into(),
        mapping: "arrows".into(),
    };
    let mut input = gui::input_mapping::InputMapping::default();
    input.configure(&controls, &h.state().player.catalog.mappings);
    input.update_devices(vec![gui::input_mapping::GamepadInfo {
        identity: "selected".into(),
        name: "Main pad".into(),
    }]);
    input.gamepad("selected", &[0.0; 4], &[]);
    assert!(input
        .gamepad("selected", &[0.0, 0.0, -0.48, 0.0], &[])
        .is_empty());
    assert_eq!(
        input.gamepad("selected", &[0.0, 0.0, -0.8, 0.0], &[]),
        vec![gui::host_input::HostInputEvent::Joystick {
            port: 1,
            up: false,
            down: false,
            left: true,
            right: false,
            fire1: false,
            fire2: false,
        }]
    );
}

#[test]
fn mapping_deletion_lists_named_and_retained_session_uses_and_preserves_surviving_ids() {
    let mut h = editor();
    h.state_mut()
        .player
        .active_configuration
        .as_mut()
        .unwrap()
        .name = "Retained session".into();
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    let mut other = draft.configurations[0].clone();
    other.id = "other".into();
    other.name = "Controller profile".into();
    other.controls.ports[1].source = InputSource::Gamepad {
        identity: "saved-pad".into(),
        mapping: "arrows".into(),
    };
    draft.configurations.push(other);
    common::settle_harness(&mut h);
    click_binding_control(&mut h, "Arrow keys");
    assert!(h
        .get_by_label("Delete mapping")
        .accesskit_node()
        .is_disabled());
    h.get_by_label("Configuration: Standard A500");
    h.get_by_label("Configuration: Controller profile");
    h.get_by_label("Active session: Retained session");
    click_binding_control(&mut h, "Arrow keys");

    click_binding_control(&mut h, "WASD");
    click_binding_control(&mut h, "Delete mapping");
    click_binding_control(&mut h, "Create mapping");
    let mapping = h
        .state()
        .player
        .editor
        .as_ref()
        .unwrap()
        .catalog
        .mappings
        .last()
        .unwrap()
        .id
        .clone();
    click_binding_control(&mut h, "Delete mapping");
    click_binding_control(&mut h, "Create mapping");
    let draft = &h.state().player.editor.as_ref().unwrap().catalog;
    assert_eq!(draft.mappings[0].id, "arrows");
    assert_ne!(draft.mappings.last().unwrap().id, mapping);
    click_binding_control(&mut h, "Cancel");
    assert!(h.state().player.catalog.mapping("wasd").is_some());
    assert!(h.state().player.catalog.mapping(&mapping).is_none());
}
