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
    app.player.active_configuration = Some(NamedConfiguration::default());
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(480.0, 640.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
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
    catalog.configurations[0].controls.host.keyboard = vec![(0x20, 0x21)];
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
        .source = source.clone();
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .controls
        .ports[1]
        .source = source;
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
        let machine = common::snapshot(h.state_mut());
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
        assert_eq!(h.state().host.status.cck, machine.cck);
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
        if !shutdown {
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
    app.player.active_configuration = Some(NamedConfiguration::default());
    app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut app, &owner_context);
    let mut h = Harness::builder()
        .with_size(egui::vec2(480.0, 640.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
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
        let _ = owner_context.run(egui::RawInput::default(), |_| {});
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
