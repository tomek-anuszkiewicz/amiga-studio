//! Framework persistence and update entry points.

use crate::app::{EmulatorApp, RTC_BATTERY_STORAGE_KEY};

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update_ui(ctx);
        if self.player.settings_dirty {
            if let Some(storage) = frame.storage_mut() {
                self.persist_player_settings(storage);
                storage.flush();
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.persist_player_settings(storage);
        eframe::set_value(storage, RTC_BATTERY_STORAGE_KEY, &self.host.status.battery);
        eframe::set_value(storage, eframe::APP_KEY, &self.preferences());
        #[cfg(not(target_arch = "wasm32"))]
        eframe::set_value(
            storage,
            crate::file_actions::HISTORY_STORAGE_KEY,
            &self.file_history,
        );
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}

impl EmulatorApp {
    fn persist_player_settings(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            crate::player_config::CONFIG_STORAGE_KEY,
            &self.player.catalog,
        );
        self.player.settings_dirty = false;
    }
}
