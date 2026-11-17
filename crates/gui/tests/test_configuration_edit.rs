#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use debugger::launch::DeviceKind;
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::player_config::{InputSource, NamedConfiguration, PortBinding};
use gui::{EmulatorApp, ViewMode};

fn editor() -> Harness<'static, EmulatorApp> {
    let mut session = Box::new(debugger::DebuggerSession::new());
    session.load_binary(0xc00000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.player.active_configuration = Some(NamedConfiguration::default());
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(480.0, 640.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut h);
    h
}

#[test]
fn cancel_discards_new_profile_shared_mapping_and_invalid_edits() {
    let mut h = editor();
    let catalog = h.state().player.catalog.clone();
    let machine = common::snapshot(h.state_mut());
    h.get_by_label("Create configuration").click();
    common::settle_harness(&mut h);
    let draft = &mut h.state_mut().player.editor.as_mut().unwrap().catalog;
    draft.mappings[0].keys[0] = 0x11;
    draft.mappings.push(gui::player_config::JoystickMapping {
        id: "created".into(),
        ..Default::default()
    });
    draft.configurations[0].name.clear();
    h.get_by_label("Cancel").click();
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
    h.get_by_label("Save").click();
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
    h.get_by_label("Save").click();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_some());
    h.get_by_label("Enter a configuration name");
    assert_eq!(h.state().player.catalog, committed);
    assert_eq!(common::snapshot(h.state_mut()), machine);
    h.get_by_label("Cancel").click();
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
        h.get_by_label("Save").click();
        common::settle_harness(&mut h);
        h.get_by_label("Mappings require unique identities and a name");
        assert!(h.state().player.editor.is_some());
        assert_eq!(h.state().player.catalog, committed);
        assert_eq!(h.state().player.active_configuration, controls);
        assert_eq!(h.state().host.status.generation, generation);
        assert_eq!(common::snapshot(h.state_mut()), machine);
        h.get_by_label("Cancel").click();
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
        h.get_by_label("Return to session").click();
        common::settle_harness(&mut h);
        h.get_by_label("Discard").click();
    } else {
        h.get_by_label("Debug").click();
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
    h.get_by_label("Cancel").click();
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
        h.get_by_label("Manual setup").click();
        common::settle_harness(&mut h);
        assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
        h.get_by_label("Unsaved configuration changes");
        // The last Save belongs to the modal; the footer remains behind it.
        h.get_all_by_label(action)
            .filter(|node| node.role() == egui::accesskit::Role::Button)
            .last()
            .unwrap()
            .click();
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
    h.get_by_label("Discard").click();
    common::settle_harness(&mut h);
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

#[test]
fn return_still_validates_host_sources_and_clean_debug_navigation_leaves_editor() {
    let mut h = editor();
    h.get_by_label("Debug").click();
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
