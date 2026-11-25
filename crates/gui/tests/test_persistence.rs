#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Automated Verification of UI Persistence via eframe::Storage
//!
//! Tests that user preferences (Theme, ViewMode, Microcode Visibility, Temporal Capacity)
//! roundtrip cleanly across application lifecycles, while emulation state (RAM, Registers,
//! Trace, Execution count) remains strictly clean and transient.

mod common;
use common::TestContext as _;
use eframe::Storage;
use egui_kittest::kittest::NodeT;
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

    fn remove_string(&mut self, key: &str) {
        self.data.remove(key);
    }

    fn flush(&mut self) {}
}

#[test]
fn game_splitter_width_roundtrips_through_host_memory_and_clamps_on_resize() {
    use egui::accesskit::{Orientation, Role};
    use egui_kittest::{kittest::Queryable, Harness};

    let game_app = || {
        let mut app = EmulatorApp::default();
        app.set_view_mode(ViewMode::ScreenOnly);
        app
    };
    let panel_edge = |h: &Harness<'_, EmulatorApp>| {
        let splitter = h.get_by_role(Role::Splitter);
        assert_eq!(
            splitter.accesskit_node().orientation(),
            Some(Orientation::Vertical)
        );
        splitter.rect().center()
    };
    let check_display = |h: &Harness<'_, EmulatorApp>| {
        let info = h.state().host.presented.unwrap();
        let image = h
            .output()
            .shapes
            .iter()
            .find_map(|clipped| {
                if let egui::Shape::Mesh(mesh) = &clipped.shape {
                    let bounds = mesh.calc_bounds();
                    (bounds.width() > 50.0 && clipped.clip_rect.expand(0.01).contains_rect(bounds))
                        .then_some(bounds)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(image.right() < panel_edge(h).x);
        assert!(
            (image.width() / image.height() - info.width as f32 / info.height as f32).abs() < 0.001
        );
    };
    let size = egui::vec2(1280.0, 800.0);
    let mut h = Harness::builder()
        .with_size(size)
        .build_ui_state(common::show_full_viewport, game_app());
    common::settle_harness(&mut h);
    let machine = common::snapshot(h.state_mut());
    common::wait_harness(&mut h, |app| app.host.presented.is_some());
    let original = panel_edge(&h);
    assert!((size.x - original.x - 220.0).abs() <= 1.0);
    let moved = original - egui::vec2(207.0, 0.0);
    h.hover_at(original);
    h.step();
    h.drag_at(original);
    h.step();
    h.hover_at(moved);
    h.step();
    h.drop_at(moved);
    common::settle_harness(&mut h);
    let saved_edge = panel_edge(&h);
    assert!((saved_edge.x - moved.x).abs() <= 2.0);
    check_display(&h);
    assert_eq!(common::snapshot(h.state_mut()), machine);

    // Round-trip through eframe's native storage key for persisted egui memory.
    let mut storage = MemoryStorage::new();
    h.ctx
        .memory(|memory| eframe::set_value(&mut storage, "egui", memory));
    let memory: egui::Memory = eframe::get_value(&storage, "egui").unwrap();
    let mut reopened = Harness::builder()
        .with_size(size)
        .with_context_setup(move |ctx| ctx.memory_mut(|target| *target = memory.clone()))
        .build_ui_state(common::show_full_viewport, game_app());
    common::settle_harness(&mut reopened);
    common::wait_harness(&mut reopened, |app| app.host.presented.is_some());
    assert!((panel_edge(&reopened).x - saved_edge.x).abs() <= 1.0);
    reopened.set_size(egui::vec2(900.0, size.y));
    common::settle_harness(&mut reopened);
    let bounded = panel_edge(&reopened);
    assert!((900.0 - bounded.x - 405.0).abs() <= 2.0);
    check_display(&reopened);
    assert!(reopened
        .ctx
        .content_rect()
        .contains_rect(reopened.get_by_label("Resume").rect()));
    reopened.set_size(egui::vec2(740.0, size.y));
    common::settle_harness(&mut reopened);
    assert!(reopened.query_by_role(Role::Splitter).is_none());
    reopened.get_by_label("Game controls").click_accesskit();
    common::settle_harness(&mut reopened);
    assert!(reopened
        .ctx
        .content_rect()
        .contains_rect(reopened.get_by_label("Resume").rect()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn library_directory_cancel_discards_the_draft_and_save_reopens_the_committed_root() {
    use eframe::App;
    use egui_kittest::{kittest::Queryable, Harness};
    use gui::player::PlayerScreen;

    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let mut draft = gui::library_edit::GameDraft::new("standard".into());
    draft.name = "Second library game".into();
    draft.save(second.path(), [true; 4]).unwrap();
    let mut app = EmulatorApp::default();
    app.player.library_root = Some(first.path().to_path_buf());
    app.navigate_player(PlayerScreen::Library);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1024.0, 760.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h.get_by_label("Configurations").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Choose library directory...").is_some());
    assert!(h
        .get_all_by_value(first.path().to_str().unwrap())
        .next()
        .is_some());
    let second_path = second.path().to_string_lossy();
    h.state_mut().player.editor.as_mut().unwrap().library_root = Some(second.path().to_path_buf());
    h.step();
    assert!(h.get_all_by_value(&second_path).next().is_some());
    h.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.library_root.as_deref(), Some(first.path()));

    common::select_library(&mut h, second.path());
    let mut storage = MemoryStorage::new();
    h.state_mut().save(&mut storage);
    let mut creation = eframe::CreationContext::_new_kittest(egui::Context::default());
    creation.storage = Some(&storage);
    let mut restarted = EmulatorApp::new(&creation);
    common::wait(&mut restarted, &creation.egui_ctx, |app| {
        app.player.library.games.len() == 1
    });
    assert_eq!(
        restarted.player.library_root.as_deref(),
        Some(second.path())
    );
    assert_eq!(restarted.player.screen, PlayerScreen::Library);
    assert_eq!(
        restarted.player.library.games[0].manifest.title,
        "Second library game"
    );
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
            media: gui::host_commands::RestoreMedia::default(),
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
        .run_test_ui(egui::RawInput::default(), |ui| restarted.update_ui(ui));
    common::command(&mut restarted, HostCommand::ReadCpu);
    let upper = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!((lower - 123..=upper - 123).contains(&restarted.host.status.rtc_time));
    assert_eq!(restarted.host.status.cck, before_cck);
}

#[test]
fn native_catalog_and_host_preferences_survive_separately_after_relocating_the_library() {
    use eframe::App;
    use gui::player::PlayerScreen;
    let original = tempfile::tempdir().unwrap();
    let root = original.path().join("Library");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("kick.rom"), [1; 262144]).unwrap();
    let mut app = EmulatorApp::default();
    app.player.library_root = Some(root.clone());
    app.theme = AppTheme::Light;
    app.navigate_player(PlayerScreen::Configuration);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    let draft = &mut app.player.editor.as_mut().unwrap().catalog;
    draft.configurations[0].name = "Renamed preset".into();
    draft.configurations[0].launch.rom = "kick.rom".into();
    draft.mappings[0].threshold = 0.7;
    app.player.rom = Some(vec![1; 262144]);
    app.save_configuration_edit();
    common::settle(&mut app, &ctx);
    assert!(app.player.editor.is_none());
    let mut storage = MemoryStorage::new();
    app.save(&mut storage);
    assert!(storage
        .get_string(gui::player_config::CONFIG_STORAGE_KEY)
        .is_none());
    let host = storage.get_string("native_player_settings").unwrap();
    assert!(!host.contains("Renamed preset") && !host.contains("kick.rom"));
    let persisted = std::fs::read(root.join("configurations.json")).unwrap();
    let text = std::str::from_utf8(&persisted).unwrap();
    assert!(text.contains("\"kick.rom\""));
    assert!(!text.contains("library_root") && text.len() < 4096);
    let reopened = reopen(&storage);
    assert_eq!(
        reopened.player.library_root.as_deref(),
        Some(root.as_path())
    );
    assert_eq!(reopened.theme, AppTheme::Light);
    assert_eq!(reopened.player.catalog, app.player.catalog);
    assert_eq!(reopened.player.catalog.configurations[0].id, "standard");
    assert!(reopened.player.rom.is_none());
    assert!(reopened.player.setup_disks.is_empty());

    let destination = tempfile::tempdir().unwrap();
    let moved = destination.path().join("Moved library");
    std::fs::rename(&root, &moved).unwrap();
    let mut different_user = EmulatorApp::default();
    different_user.navigate_player(PlayerScreen::Configuration);
    let mut h = egui_kittest::Harness::builder().build_ui_state(
        |ui, app: &mut EmulatorApp| app.update_ui(ui),
        different_user,
    );
    common::settle_harness(&mut h);
    common::select_library(&mut h, &moved);
    let reopened = h.state();
    assert_eq!(reopened.player.catalog, app.player.catalog);
    assert_eq!(
        reopened.player.catalog.selected().unwrap().name,
        "Renamed preset"
    );
    assert_eq!(reopened.theme, EmulatorApp::default().theme);
    assert_eq!(std::fs::read(moved.join("kick.rom")).unwrap(), [1; 262144]);
}

fn reopen(storage: &dyn Storage) -> EmulatorApp {
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(storage);
    EmulatorApp::new(&context)
}

fn legacy_settings(root: &std::path::Path, rom: &std::path::Path) -> MemoryStorage {
    use gui::player_config::{
        ConfigurationCatalog, JoystickMapping, NamedConfiguration, CONFIG_STORAGE_KEY,
    };
    #[derive(serde::Serialize)]
    struct Legacy<'a> {
        configurations: Vec<NamedConfiguration>,
        mappings: Vec<JoystickMapping>,
        selected: &'a str,
        library_root: &'a std::path::Path,
    }
    let mut catalog = ConfigurationCatalog::default();
    catalog.mappings.truncate(1);
    catalog.mappings[0].name = "Arrow keys and numpad fire".into();
    catalog.mappings[0].keys[4..].copy_from_slice(&[0x0f, 0x3c]);
    catalog.configurations[0].launch.rom = rom.to_str().unwrap().into();
    let mut profile = catalog.configurations[0].clone();
    profile.id = "played-profile".into();
    profile.name = "My A500".into();
    catalog.configurations.push(profile);
    catalog.mappings.push(JoystickMapping {
        id: "extra".into(),
        ..JoystickMapping::default()
    });
    let mut storage = MemoryStorage::new();
    eframe::set_value(
        &mut storage,
        CONFIG_STORAGE_KEY,
        &Legacy {
            configurations: catalog.configurations,
            mappings: catalog.mappings,
            selected: "played-profile",
            library_root: root,
        },
    );
    storage
}

#[test]
fn legacy_catalog_migrates_once_and_survives_restart_before_the_host_flush() {
    use eframe::App;
    use gui::player_config::CONFIG_STORAGE_KEY;
    let root = tempfile::tempdir().unwrap();
    let assets = root.path().join("_shared");
    std::fs::create_dir(&assets).unwrap();
    let rom = assets.join("firmware.rom");
    std::fs::write(&rom, [7; 262144]).unwrap();
    let mut storage = legacy_settings(root.path(), &rom);
    let original = storage.get_string(CONFIG_STORAGE_KEY).unwrap();
    let migrated = reopen(&storage);
    let catalog = migrated.player.catalog.clone();
    assert_eq!(catalog.selected, "played-profile");
    assert_eq!(
        catalog
            .configurations
            .iter()
            .map(|cfg| cfg.id.as_str())
            .collect::<Vec<_>>(),
        ["standard", "played-profile"]
    );
    assert!(catalog
        .configurations
        .iter()
        .all(|cfg| cfg.launch.rom == "_shared/firmware.rom"));
    assert_eq!(
        catalog
            .mappings
            .iter()
            .map(|mapping| mapping.id.as_str())
            .collect::<Vec<_>>(),
        ["arrows", "extra", "wasd"]
    );
    assert_eq!(catalog.mapping("arrows").unwrap().keys[4..], [0x31, 0x32]);
    let installed = root.path().join("configurations.json");
    let installed_bytes = std::fs::read(&installed).unwrap();
    assert_eq!(
        std::fs::read_dir(root.path()).unwrap().count(),
        2,
        "only the assets and portable catalog belong in the library"
    );
    assert_eq!(std::fs::read(&rom).unwrap(), [7; 262144]);

    // Simulate an exit before the first app writes its per-user migration marker.
    let mut restarted = reopen(&storage);
    assert_eq!(restarted.player.catalog, catalog);
    assert_eq!(std::fs::read(&installed).unwrap(), installed_bytes);
    restarted.save(&mut storage);
    assert_eq!(
        storage.get_string(CONFIG_STORAGE_KEY).as_deref(),
        Some(original.as_str())
    );

    let mut edited: serde_json::Value = serde_json::from_slice(&installed_bytes).unwrap();
    edited["configurations"][1]["name"] = "Edited after migration".into();
    std::fs::write(&installed, serde_json::to_vec(&edited).unwrap()).unwrap();
    assert_eq!(
        reopen(&storage).player.catalog.selected().unwrap().name,
        "Edited after migration"
    );
}

#[test]
fn failed_migration_preserves_the_exact_legacy_data_and_occupied_library() {
    use eframe::App;
    use gui::player_config::CONFIG_STORAGE_KEY;
    let root = tempfile::tempdir().unwrap();
    let rom = root.path().join("firmware.rom");
    std::fs::write(&rom, [0; 262144]).unwrap();
    let installed = root.path().join("configurations.json");
    let existing = br#"{"configurations":[],"mappings":[]}"#;
    std::fs::write(&installed, existing).unwrap();
    let mut storage = legacy_settings(root.path(), &rom);
    let original = storage.get_string(CONFIG_STORAGE_KEY).unwrap();
    let mut app = reopen(&storage);
    assert!(app.player.catalog.configurations.is_empty());
    app.save(&mut storage);
    assert_eq!(std::fs::read(&installed).unwrap(), existing);
    assert_eq!(storage.get_string(CONFIG_STORAGE_KEY).unwrap(), original);
    let mut h = egui_kittest::Harness::builder()
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    use egui_kittest::kittest::Queryable;
    common::settle_harness(&mut h);
    assert!(h
        .query_all_by(|node| node
            .value()
            .is_some_and(|value| value.contains("already exists in the previous library")))
        .next()
        .is_some());
    assert!(h.get_by_label("Add game").accesskit_node().is_disabled());
}

#[test]
fn legacy_migration_rejects_an_external_rom_without_creating_a_catalog() {
    use gui::player_config::CONFIG_STORAGE_KEY;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let rom = external.path().join("firmware.rom");
    std::fs::write(&rom, [4; 262144]).unwrap();
    let storage = legacy_settings(root.path(), &rom);
    let original = storage.get_string(CONFIG_STORAGE_KEY).unwrap();
    let app = reopen(&storage);
    assert!(app.player.catalog.configurations.is_empty());
    assert!(!root.path().join("configurations.json").exists());
    assert_eq!(storage.get_string(CONFIG_STORAGE_KEY).unwrap(), original);
    assert_eq!(std::fs::read(&rom).unwrap(), [4; 262144]);
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
    let library = tempfile::tempdir().unwrap();
    app.player.library_root = Some(library.path().to_path_buf());
    app.navigate_player(gui::player::PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 1800.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h.get_by_label("Create configuration").click_accesskit();
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
    let name_editor = h
        .get_all_by_value("New configuration")
        .into_iter()
        .find(|node| node.accesskit_node().role() == egui::accesskit::Role::TextInput)
        .unwrap();
    name_editor.focus();
    name_editor.type_text(" edited");
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
    h.get_by_label("DF1: connected").click_accesskit();
    h.get_by_label("Create mapping").click_accesskit();
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
        3
    );
    assert_ne!(
        h.state().player.editor.as_ref().unwrap().catalog.mappings[0].id,
        h.state().player.editor.as_ref().unwrap().catalog.mappings[2].id
    );
    let committed_before = h.state().player.catalog.clone();
    let mut draft_storage = MemoryStorage::new();
    h.state_mut().save(&mut draft_storage);
    let stored_before = reopen(&draft_storage).player.catalog;
    assert_eq!(
        stored_before, committed_before,
        "shutdown persisted a draft"
    );
    h.get_by_label("Save").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    let mut storage = MemoryStorage::new();
    h.state_mut().save(&mut storage);
    let restored = reopen(&storage).player.catalog;
    assert_eq!(restored, h.state().player.catalog);
    assert_eq!(restored.selected, selected);
    assert_eq!(
        restored.mappings[2].keys,
        [0x11, 0x21, 0x20, 0x22, 0x26, 0x27]
    );
    assert!(!restored.configurations[0].launch.drives[1]);
}

#[test]
fn saved_catalog_introduces_presets_once_and_keeps_customized_controls() {
    use eframe::App;
    use gui::player::PlayerScreen;
    use gui::player_config::{ConfigurationCatalog, InputSource};

    for customized in [false, true] {
        let library = tempfile::tempdir().unwrap();
        let catalog_path = library.path().join("configurations.json");
        let mut storage = MemoryStorage::new();
        let mut older = ConfigurationCatalog::default();
        older.mappings.truncate(1);
        older.mappings[0].name = "Arrow keys and numpad fire".into();
        older.mappings[0].keys[4..].copy_from_slice(&[0x0f, 0x3c]);
        if customized {
            older.mappings[0].name = "My joystick".into();
            older.mappings[0].threshold = 0.7;
        }
        let mut custom_profile = older.configurations[0].clone();
        custom_profile.id = "custom".into();
        older.selected = custom_profile.id.clone();
        older.configurations.push(custom_profile);
        let original = serde_json::to_vec(&serde_json::json!({
            "configurations": older.configurations,
            "mappings": older.mappings,
        }))
        .unwrap();
        std::fs::write(&catalog_path, &original).unwrap();
        let mut previous = EmulatorApp::default();
        previous.player.library_root = Some(library.path().to_path_buf());
        previous.player.catalog.selected = older.selected.clone();
        previous.save(&mut storage);

        let mut app = reopen(&storage);
        let catalog = &app.player.catalog;
        assert_eq!(catalog.selected, "custom");
        assert_eq!(
            catalog.selected().unwrap().controls.ports[1].source,
            InputSource::Keyboard("arrows".into())
        );
        if customized {
            assert_eq!(catalog.mapping("arrows"), older.mapping("arrows"));
            assert_eq!(catalog.mappings.len(), 3);
        } else {
            assert_eq!(catalog.mapping("arrows").unwrap().keys[4..], [0x31, 0x32]);
            assert_eq!(catalog.mappings.len(), 2);
        }
        assert_eq!(
            catalog
                .mappings
                .iter()
                .filter(|mapping| mapping.name == "Arrow keys")
                .count(),
            1
        );
        assert_eq!(
            catalog
                .mappings
                .iter()
                .filter(|mapping| mapping.name == "WASD")
                .count(),
            1
        );
        app.save(&mut storage);
        assert_eq!(std::fs::read(&catalog_path).unwrap(), original);
        assert_eq!(reopen(&storage).player.catalog, app.player.catalog);

        app.navigate_player(PlayerScreen::Configuration);
        let ctx = egui::Context::default();
        common::settle(&mut app, &ctx);
        app.player.editor.as_mut().unwrap().catalog.configurations[1].name =
            "Updated configuration".into();
        app.save_configuration_edit();
        common::settle(&mut app, &ctx);
        assert!(app.player.editor.is_none());
        assert_ne!(std::fs::read(&catalog_path).unwrap(), original);
        app.save(&mut storage);
        assert_eq!(reopen(&storage).player.catalog, app.player.catalog);
    }
}

#[test]
fn editor_mapping_deletion_and_id_allocation_survive_storage_and_reopen() {
    use eframe::App;
    use egui_kittest::{kittest::Queryable, Harness};
    use gui::player::PlayerScreen;

    let mut app = EmulatorApp::default();
    let library = tempfile::tempdir().unwrap();
    app.player.library_root = Some(library.path().to_path_buf());
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h.get_by_label("WASD").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Delete mapping").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Create mapping").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Delete mapping").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Save").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().player.catalog.mappings.len(), 1);

    let mut storage = MemoryStorage::new();
    h.state_mut().save(&mut storage);
    let mut creation = eframe::CreationContext::_new_kittest(egui::Context::default());
    creation.storage = Some(&storage);
    let mut reopened = EmulatorApp::new(&creation);
    assert!(reopened.player.catalog.mapping("wasd").is_none());
    assert!(reopened.player.catalog.mapping("mapping-1").is_none());
    assert_eq!(
        reopened.player.catalog.mapping("arrows"),
        h.state().player.catalog.mapping("arrows")
    );
    reopened.navigate_player(PlayerScreen::Configuration);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), reopened);
    common::settle_harness(&mut h);
    h.get_by_label("Create mapping").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().player.editor.as_ref().unwrap().catalog.mappings[1].id,
        "mapping-2"
    );
}
