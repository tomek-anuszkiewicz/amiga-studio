#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Automated Verification of UI Persistence via eframe::Storage
//!
//! Tests that user preferences (Theme, ViewMode, Microcode Visibility, Temporal Capacity)
//! roundtrip cleanly across application lifecycles, while emulation state (RAM, Registers,
//! Trace, Execution count) remains strictly clean and transient.

mod common;
use eframe::Storage;
use gui::host_commands::HostCommand;
use gui::{AppTheme, EmulatorApp, ScreenScale, UserPreferences, ViewMode};
use std::collections::HashMap;

/// In-memory implementation of eframe::Storage for headless deterministic testing
struct MemoryStorage {
    data: HashMap<String, String>,
}

impl MemoryStorage {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), value);
    }

    fn flush(&mut self) {}
}

#[test]
fn test_user_preferences_roundtrip_via_storage() {
    let mut storage = MemoryStorage::new();

    // 1. Create app with modified user preferences
    let mut session = Box::new(gui::DebuggerSession::new());
    session.temporal.set_capacity(50_000);
    session.load_binary(0x001000, &[0x4E, 0x71], true).unwrap();
    session.machine.cpu.state.set_d_long(0, 0x12345678);
    session.instructions_executed = 42;
    let mut app = EmulatorApp::with_session(session);
    app.theme = AppTheme::ClassicWorkbench;
    app.view_mode = ViewMode::ScreenOnly;
    app.screen_scale = ScreenScale::Two;
    app.show_microcode = false;
    app.temporal_capacity_selection = 50_000;
    app.disasm_pane_width = 520.0;
    app.crt_pane_height = 520.0;
    app.right_dock_bottom_height = 310.0;
    // 2. Persist state via eframe::App::save
    use eframe::App;
    app.save(&mut storage);

    // Verify storage has been populated under APP_KEY
    assert!(storage.get_string(eframe::APP_KEY).is_some());

    // 3. Create a fresh session and simulate restart loading from storage
    let loaded_prefs: Option<UserPreferences> = eframe::get_value(&storage, eframe::APP_KEY);
    assert!(
        loaded_prefs.is_some(),
        "Expected saved UserPreferences in storage"
    );

    let prefs = loaded_prefs.unwrap();
    assert_eq!(prefs.theme, AppTheme::ClassicWorkbench);
    assert_eq!(prefs.view_mode, ViewMode::ScreenOnly);
    assert_eq!(prefs.screen_scale, ScreenScale::Two);
    assert_eq!(prefs.show_microcode, false);
    assert_eq!(prefs.temporal_capacity, 50_000);
    assert_eq!(prefs.disasm_pane_width, 520.0);
    assert_eq!(prefs.crt_pane_height, 520.0);
    assert_eq!(prefs.right_dock_bottom_height, 310.0);

    let mut new_app = EmulatorApp::default();
    new_app.apply_preferences(&prefs);

    // 4. Assert UI preferences are restored
    assert_eq!(new_app.theme, AppTheme::ClassicWorkbench);
    assert_eq!(new_app.view_mode, ViewMode::ScreenOnly);
    assert_eq!(new_app.screen_scale, ScreenScale::Two);
    assert_eq!(new_app.show_microcode, false);
    assert_eq!(new_app.temporal_capacity_selection, 50_000);
    assert_eq!(new_app.disasm_pane_width, 520.0);
    assert_eq!(new_app.crt_pane_height, 520.0);
    assert_eq!(new_app.right_dock_bottom_height, 310.0);

    // 5. Assert Guest Emulation State is CLEAN & TRANSIENT (never persisted)
    let mut fresh_app = EmulatorApp::default();
    let fresh = common::inspect(&mut fresh_app);
    let restored = common::inspect(&mut new_app);
    assert_eq!(
        restored.cpu.d_long(0),
        fresh.cpu.d_long(0),
        "Guest registers must not be persisted"
    );
    assert_eq!(
        restored.instructions_executed, 0,
        "Instruction count must start at 0"
    );
    assert_eq!(
        restored.cpu.pc(),
        fresh.cpu.pc(),
        "PC must remain fresh default on restart"
    );
    assert_ne!(
        restored.cpu.pc(),
        0x001000,
        "Loaded binary PC must not be persisted"
    );
}

#[test]
fn test_default_preferences_fallback_when_storage_empty() {
    let storage = MemoryStorage::new();
    let loaded_prefs: Option<UserPreferences> = eframe::get_value(&storage, eframe::APP_KEY);
    assert!(loaded_prefs.is_none());

    let default_prefs = UserPreferences::default();
    assert_eq!(default_prefs.theme, AppTheme::Dark);
    assert_eq!(default_prefs.view_mode, ViewMode::Developer);
    assert_eq!(default_prefs.show_microcode, true);
    assert_eq!(default_prefs.temporal_capacity, 25_000);
}

#[test]
fn test_persist_egui_memory_enabled() {
    use eframe::App;
    let app = EmulatorApp::default();
    assert!(
        app.persist_egui_memory(),
        "persist_egui_memory must be true so splitter positions and collapsing headers persist"
    );
}

#[test]
fn test_app_session_save_state_file_json_roundtrip() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.machine.step_cycles(100);
    let mut app = EmulatorApp::with_session(session);
    let temp_file =
        std::env::temp_dir().join(format!("gui_test_save_state_{}.json", std::process::id()));
    app.save_state_file(&temp_file);
    common::settle(&mut app, &egui::Context::default());
    assert!(app.file_error.is_none());
    assert!(app
        .toast_message
        .as_ref()
        .unwrap()
        .0
        .contains("State saved to"));
    let mut new_app = EmulatorApp::default();
    new_app.load_state_file(&temp_file);
    common::settle(&mut new_app, &egui::Context::default());
    assert!(new_app.file_error.is_none());
    assert_eq!(app.host.status.cck, new_app.host.status.cck);
    assert_eq!(common::snapshot(&mut app), common::snapshot(&mut new_app));
    let saved = std::fs::read(&temp_file).unwrap();
    app.save_state_file(&temp_file.join("unavailable.json"));
    common::settle(&mut app, &egui::Context::default());
    assert!(app
        .file_error
        .as_ref()
        .unwrap()
        .contains("Could not save state"));
    assert_eq!(std::fs::read(&temp_file).unwrap(), saved);
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn battery_offset_persists_separately_and_ui_refreshes_host_utc() {
    use eframe::App;
    let mut storage = MemoryStorage::new();
    let mut app = EmulatorApp::default();
    common::command(
        &mut app,
        HostCommand::LoadRtcBattery(rtc::RtcBattery {
            offset_seconds: -123,
        }),
    );
    let snapshot = common::snapshot_json(&mut app);
    app.save(&mut storage);
    let battery: rtc::RtcBattery = eframe::get_value(&storage, "rtc_battery").unwrap();
    assert_eq!(battery.offset_seconds, -123);
    let encoded = storage.get_string("rtc_battery").unwrap();
    assert!(!encoded.contains("timestamp"));
    assert!(!encoded.contains("registers"));
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(&storage);
    let mut restarted = EmulatorApp::new(&context);
    common::settle(&mut restarted, &context.egui_ctx);
    assert_eq!(restarted.host.status.battery, battery);
    assert!(common::command(
        &mut restarted,
        HostCommand::RestorePlayer {
            json: snapshot,
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick
            ],
            resume: false
        }
    )
    .value
    .is_ok());
    assert_eq!(restarted.host.status.battery, battery);
    // Model an old sample while the UI is paused, then refresh without stepping.
    common::command(&mut restarted, HostCommand::SupplyUtc(631152000));
    let before_cck = restarted.host.status.cck;
    let lower = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let _ = context
        .egui_ctx
        .run(egui::RawInput::default(), |ctx| restarted.update_ui(ctx));
    common::command(&mut restarted, HostCommand::ReadCpu);
    let upper = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!((lower - 123..=upper - 123).contains(&restarted.host.status.rtc_time));
    assert_eq!(restarted.host.status.cck, before_cck);
}

#[test]
fn named_hardware_and_input_catalog_survives_framework_storage_without_media_bytes() {
    use eframe::App;
    let mut app = EmulatorApp::default();
    app.player.catalog.configurations[0].name = "Renamed preset".into();
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.player.catalog.mappings[0].threshold = 0.7;
    app.player.rom = Some(vec![1; 262144]);
    let mut storage = MemoryStorage::new();
    app.save(&mut storage);
    let restored: gui::player_config::ConfigurationCatalog =
        eframe::get_value(&storage, gui::player_config::CONFIG_STORAGE_KEY).unwrap();
    assert_eq!(restored, app.player.catalog);
    assert_eq!(restored.configurations[0].id, "standard");
    let mut reopened = EmulatorApp::default();
    reopened.player.catalog = restored;
    assert!(reopened.player.rom.is_none());
    assert!(reopened.player.setup_disks.is_empty());
    assert_eq!(
        reopened.player.catalog.selected().unwrap().name,
        "Renamed preset"
    );
}

#[test]
fn existing_flat_user_configuration_remains_readable_without_migration() {
    let mut storage = MemoryStorage::new();
    storage.set_string("configuration", r#"(id:"standard",name:"My existing configuration",preset:Standard1Mb,video:Pal,rom:"kick.rom",drives:(true,false,false,false),ports:((device:Mouse,source:SystemMouse),(device:Joystick,source:Keyboard("arrows"))),mouse_sensitivity:1.5,keyboard:[(32,33)],frontend:(release:"",pause:"F5",save:"F6",load:"F9"),gamepad_shortcuts:[])"#.into());
    let configuration: gui::player_config::NamedConfiguration =
        eframe::get_value(&storage, "configuration").unwrap();
    assert_eq!(configuration.id, "standard");
    assert_eq!(configuration.name, "My existing configuration");
    assert_eq!(configuration.launch.rom, "kick.rom");
    assert_eq!(configuration.controls.host.keyboard, vec![(32, 33)]);
    assert_eq!(configuration.controls.host.mouse_sensitivity, 1.5);
    eframe::set_value(&mut storage, "configuration", &configuration);
    assert!(storage
        .get_string("configuration")
        .unwrap()
        .contains("preset:Standard1Mb"));
    assert_eq!(
        eframe::get_value::<gui::player_config::NamedConfiguration>(&storage, "configuration")
            .unwrap(),
        configuration
    );
}

#[test]
fn configuration_creation_mapping_and_hardware_edits_persist_through_real_controls() {
    use eframe::App;
    use egui_kittest::{kittest::Queryable, Harness};
    let mut app = EmulatorApp::default();
    app.navigate_player(gui::player::PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 1800.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut h);
    h.get_by_label("Create configuration").click();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state()
            .player
            .editor
            .as_ref()
            .unwrap()
            .catalog
            .configurations
            .len(),
        2
    );
    let selected = h
        .state()
        .player
        .editor
        .as_ref()
        .unwrap()
        .catalog
        .selected
        .clone();
    assert_ne!(selected, "standard");
    h.get_all_by_value("New configuration")
        .into_iter()
        .find(|node| node.role() == egui::accesskit::Role::TextInput)
        .unwrap()
        .type_text(" edited");
    common::settle_harness(&mut h);
    assert!(h
        .state()
        .player
        .editor
        .as_ref()
        .unwrap()
        .catalog
        .selected()
        .unwrap()
        .name
        .contains("edited"));
    h.get_by_label("DF1: connected").click();
    h.get_by_label("Create mapping").click();
    common::settle_harness(&mut h);
    assert!(
        h.state()
            .player
            .editor
            .as_ref()
            .unwrap()
            .catalog
            .selected()
            .unwrap()
            .launch
            .drives[1]
    );
    assert_eq!(
        h.state()
            .player
            .editor
            .as_ref()
            .unwrap()
            .catalog
            .mappings
            .len(),
        2
    );
    assert_ne!(
        h.state().player.editor.as_ref().unwrap().catalog.mappings[0].id,
        h.state().player.editor.as_ref().unwrap().catalog.mappings[1].id
    );
    let committed_before = h.state().player.catalog.clone();
    let mut draft_storage = MemoryStorage::new();
    h.state_mut().save(&mut draft_storage);
    let stored_before: gui::player_config::ConfigurationCatalog =
        eframe::get_value(&draft_storage, gui::player_config::CONFIG_STORAGE_KEY).unwrap();
    assert_eq!(
        stored_before, committed_before,
        "shutdown persisted a draft"
    );
    h.get_by_label("Save").click();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    let mut storage = MemoryStorage::new();
    h.state_mut().save(&mut storage);
    let restored: gui::player_config::ConfigurationCatalog =
        eframe::get_value(&storage, gui::player_config::CONFIG_STORAGE_KEY).unwrap();
    assert_eq!(restored, h.state().player.catalog);
    assert_eq!(restored.selected, selected);
    assert_eq!(
        restored.mappings[1].keys,
        [0x11, 0x21, 0x20, 0x22, 0x40, 0x33]
    );
    assert!(!restored.configurations[0].launch.drives[1]);
}
