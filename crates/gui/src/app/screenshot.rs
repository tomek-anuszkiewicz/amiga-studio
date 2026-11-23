use super::EmulatorApp;
use crate::game_library::{persist_screenshot, LibraryGame};
use crate::layout::top_menu_bar::TOAST_DURATION_FRAMES;
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use std::io::{Error, Write};

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
        let [width, height] = image.size;
        let rgba: Vec<_> = image
            .pixels
            .iter()
            .flat_map(egui::Color32::to_srgba_unmultiplied)
            .collect();
        match persist_screenshot(game, |file| {
            // The encoder writes its final chunk on drop. Encode into memory so
            // every byte reaches the file through a write that reports errors.
            let mut png = Vec::new();
            PngEncoder::new(&mut png)
                .write_image(&rgba, width as u32, height as u32, ExtendedColorType::Rgba8)
                .map_err(Error::other)?;
            file.write_all(&png)
        }) {
            Ok(()) => {
                self.toast_message = Some(("Screenshot saved".into(), TOAST_DURATION_FRAMES));
            }
            Err(error) => self.file_error = Some(format!("Could not save screenshot: {error}")),
        }
    }
}
