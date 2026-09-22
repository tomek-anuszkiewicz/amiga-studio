#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Automated Verification of UI Persistence via eframe::Storage
//!
//! Tests that user preferences (Theme, ViewMode, Microcode Visibility, Temporal Capacity)
//! roundtrip cleanly across application lifecycles, while emulation state (RAM, Registers,
//! Trace, Execution count) remains strictly clean and transient.

use eframe::Storage;
use gui::{AppTheme, EmulatorApp, UserPreferences, ViewMode};
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
    let mut app = EmulatorApp::default();
    app.theme = AppTheme::ClassicWorkbench;
    app.view_mode = ViewMode::ScreenOnly;
    app.show_microcode = false;
    app.temporal_capacity_selection = 50_000;
    app.disasm_pane_width = 520.0;
    app.crt_pane_height = 520.0;
    app.right_dock_bottom_height = 310.0;
    app.session.temporal.set_capacity(50_000);

    // Also run some guest code to alter CPU/RAM state
    app.session.load_binary(0x001000, &[0x4E, 0x71], true);
    app.session.machine.cpu.state.set_d_long(0, 0x12345678);
    app.session.instructions_executed = 42;

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
    assert_eq!(new_app.show_microcode, false);
    assert_eq!(new_app.temporal_capacity_selection, 50_000);
    assert_eq!(new_app.disasm_pane_width, 520.0);
    assert_eq!(new_app.crt_pane_height, 520.0);
    assert_eq!(new_app.right_dock_bottom_height, 310.0);

    // 5. Assert Guest Emulation State is CLEAN & TRANSIENT (never persisted)
    let fresh_app = EmulatorApp::default();
    assert_eq!(
        new_app.session.machine.cpu.state.d_long(0),
        fresh_app.session.machine.cpu.state.d_long(0),
        "Guest registers must not be persisted"
    );
    assert_eq!(
        new_app.session.instructions_executed, 0,
        "Instruction count must start at 0"
    );
    assert_eq!(
        new_app.session.machine.cpu.state.pc, fresh_app.session.machine.cpu.state.pc,
        "PC must remain fresh default on restart"
    );
    assert_ne!(
        new_app.session.machine.cpu.state.pc, 0x001000,
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
