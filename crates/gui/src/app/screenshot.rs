use super::EmulatorApp;
use crate::game_library::{
    load_game, persist_cover, persist_screenshot, write_rgba_png, LibraryGame,
};

impl EmulatorApp {
    pub(crate) fn associate_library_game(&mut self, game: Option<LibraryGame>) {
        if !self
            .player
            .active_game
            .as_ref()
            .zip(game.as_ref())
            .is_some_and(|(active, launched)| active.same_entry(launched))
        {
            self.screen_texture.clear_capture();
            self.player.saves.clear();
            self.player.game_slots.fill(false);
            self.player.session_save_error = None;
        }
        self.player.active_game = game;
    }

    pub(crate) fn take_game_screenshot(&mut self) {
        let Some(game) = &self.player.active_game else {
            return;
        };
        let Some(image) = self.screen_texture.displayed() else {
            self.file_error = Some("Could not save screenshot: No game image is available".into());
            return;
        };
        if let Err(error) = persist_screenshot(game, |file| write_frame(file, image)) {
            self.file_error = Some(format!("Could not save screenshot: {error}"));
        }
    }

    pub(crate) fn capture_library_cover(&mut self) {
        let Some(game) = &self.player.active_game else {
            return;
        };
        let Some(image) = self.screen_texture.displayed() else {
            self.file_error = Some("Could not save cover: No game image is available".into());
            return;
        };
        if let Err(error) = persist_cover(game, |file| write_frame(file, image)) {
            self.file_error = Some(format!("Could not update cover: {error}"));
            return;
        }
        if let Some(entry) = self
            .player
            .library
            .games
            .iter_mut()
            .find(|entry| entry.same_entry(game))
        {
            match load_game(&game.directory) {
                Ok(updated) => {
                    *entry = updated;
                    if let Some(sync) = &mut self.player.library_sync {
                        sync.changed_locally();
                    }
                }
                Err(error) => {
                    self.file_error = Some(format!(
                        "Cover saved, but the game could not be reloaded: {error}"
                    ))
                }
            }
        }
    }
}

fn write_frame(file: &mut std::fs::File, image: &egui::ColorImage) -> std::io::Result<()> {
    let [width, height] = image.size;
    let rgba: Vec<_> = image
        .pixels
        .iter()
        .flat_map(egui::Color32::to_srgba_unmultiplied)
        .collect();
    write_rgba_png(file, &rgba, width as u32, height as u32)
}
