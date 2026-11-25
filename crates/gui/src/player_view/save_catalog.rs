use super::SAVE_DESCRIPTION_DIALOG;
use crate::game_library::{self, LibraryGame, PlayerSave};
use crate::player::save_catalog::DescriptionDraft;
use crate::EmulatorApp;
use std::path::PathBuf;

enum RowAction {
    Load(PathBuf),
    Edit(PathBuf, String),
    Delete(PathBuf),
    Undo(PathBuf),
}

impl EmulatorApp {
    pub(super) fn render_session_saves(&mut self, ui: &mut egui::Ui, game: &LibraryGame) {
        ui.separator();
        ui.heading("Saved progress");
        if let Some(latest) = self
            .player
            .saves
            .iter()
            .filter_map(PlayerSave::timestamp)
            .max()
        {
            ui.label(format!(
                "Latest save: {}",
                self.player.save_catalog.times.label(latest)
            ));
        }
        let height = ui
            .available_height()
            .min(ui.viewport_rect().height() * 0.35);
        egui::ScrollArea::vertical()
            .id_salt("session-saves")
            .max_height(height.max(60.0))
            .auto_shrink([false, true])
            .show(ui, |ui| self.render_save_catalog(ui, game, true));
    }

    pub(super) fn render_save_catalog(
        &mut self,
        ui: &mut egui::Ui,
        game: &LibraryGame,
        active: bool,
    ) {
        let (saves, error) = if active {
            (&self.player.saves, &self.player.session_save_error)
        } else {
            (
                &self.player.selected_saves,
                &self.player.selected_save_error,
            )
        };
        if let Some(error) = error {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Could not list saved progress: {error}"),
            );
        }
        if saves.is_empty() && error.is_none() {
            ui.label("No saved progress");
        }
        let mut action = None;
        let enabled = !self.player.busy && (active || self.player.library_available());
        let now = ui.input(|input| input.time);
        for save in saves {
            ui.push_id(&save.path, |ui| {
                if save.quicksave {
                    ui.strong("Quicksave");
                }
                let description = save
                    .header
                    .as_ref()
                    .and_then(|header| header.description.as_deref());
                let name = save.path.file_name().unwrap_or_default().to_string_lossy();
                if save.header.is_none() {
                    ui.label(name.as_ref());
                } else if let Some(description) = description {
                    ui.label(description);
                } else {
                    ui.label(format!("Saved progress ({name})"));
                }
                if let Some(timestamp) = save.timestamp() {
                    ui.label(self.player.save_catalog.times.label(timestamp));
                }
                if let Some(problem) = &save.problem {
                    ui.colored_label(ui.visuals().error_fg_color, problem);
                }
                save_preview(ui, save);
                if let Some(deadline) = self.player.save_catalog.deadline(game, &save.path) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "Deletes in {:.0} s",
                            (deadline - now).ceil().max(0.0)
                        ));
                        if ui
                            .button("Undo")
                            .on_hover_text("Keep this save and its current availability.")
                            .clicked()
                        {
                            action = Some(RowAction::Undo(save.path.clone()));
                        }
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        if save.header.is_some() {
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new("Load state")
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .on_hover_text("Restore this save and resume play.")
                                .clicked()
                            {
                                action = Some(RowAction::Load(save.path.clone()));
                            }
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new("Edit description")
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .on_hover_text("Change the label displayed for this save.")
                                .clicked()
                            {
                                action = Some(RowAction::Edit(
                                    save.path.clone(),
                                    description.unwrap_or_default().to_owned(),
                                ));
                            }
                        }
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new("Delete save")
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            )
                            .on_hover_text("Remove this save after a ten-second Undo window.")
                            .clicked()
                        {
                            action = Some(RowAction::Delete(save.path.clone()));
                        }
                    });
                }
                ui.separator();
            });
        }
        match action {
            Some(RowAction::Load(path)) => self.load_catalog_save(game.clone(), path, false),
            Some(RowAction::Edit(path, text)) => {
                self.player.captured = false;
                self.player.save_catalog.description = Some(DescriptionDraft {
                    game: game.clone(),
                    path,
                    text,
                });
            }
            Some(RowAction::Delete(path)) => self.begin_delete_save(ui.ctx(), game, path),
            Some(RowAction::Undo(path)) => {
                self.undo_delete_save(game, &path);
                ui.ctx().request_repaint();
            }
            None => {}
        }
    }

    pub(crate) fn render_save_description(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.player.save_catalog.description else {
            return;
        };
        let dialog = egui::Modal::new(egui::Id::unique(SAVE_DESCRIPTION_DIALOG)).show(ctx, |ui| {
            ui.heading("Edit save description");
            ui.add(
                egui::TextEdit::singleline(&mut draft.text)
                    .desired_width(270.0)
                    .hint_text("Description"),
            );
            ui.horizontal(|ui| {
                let save = ui.button("Save description").clicked();
                let cancel = ui.button("Cancel").clicked();
                (save, cancel)
            })
            .inner
        });
        let (save, cancel) = dialog.inner;
        if save {
            let result = game_library::edit_save_description(&draft.game, &draft.path, &draft.text);
            let directory = draft.game.directory.clone();
            let game_id = draft.game.manifest.id.clone();
            self.player.save_catalog.description = None;
            if let Err(error) = result {
                self.file_error = Some(format!("Could not edit save description: {error}"));
            }
            self.refresh_game_saves(&directory, &game_id);
        } else if cancel || (!dialog.backdrop_response.clicked() && dialog.should_close()) {
            self.player.save_catalog.description = None;
        }
    }
}

fn save_preview(ui: &mut egui::Ui, save: &PlayerSave) {
    let Some((dimensions, revision, png)) = save.preview() else {
        ui.label("No preview");
        return;
    };
    let width = ui
        .available_rect_before_wrap()
        .intersect(ui.clip_rect())
        .width()
        .clamp(1.0, 320.0);
    let ratio = dimensions[1] as f32 / dimensions[0] as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, width * ratio), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let pixels_per_point = ui.ctx().pixels_per_point();
    let max_edge = ui.input(|input| input.max_texture_side) as f32;
    let scale = (rect.width() * pixels_per_point / dimensions[0] as f32)
        .min(max_edge / dimensions[0].max(dimensions[1]) as f32)
        .min(1.0);
    let pixels = dimensions.map(|size| (size as f32 * scale).round().max(1.0) as u32);
    type CachedPreview = (u64, [u32; 2], Option<egui::TextureHandle>);
    let id = ui.scope_id().with("saved-thumbnail");
    let cached = ui.data(|data| data.get_temp::<CachedPreview>(id));
    let texture = if let Some((_, _, texture)) =
        cached.filter(|(rev, size, _)| *rev == revision && *size == pixels)
    {
        texture
    } else {
        let texture = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .ok()
            .map(|image| {
                let source = image.into_rgba8();
                let thumbnail = image::imageops::resize(
                    &source,
                    pixels[0],
                    pixels[1],
                    image::imageops::FilterType::Triangle,
                );
                ui.ctx().load_texture(
                    format!("Saved thumbnail {}", save.path.display()),
                    egui::ColorImage::from_rgba_unmultiplied(
                        pixels.map(|size| size as usize),
                        thumbnail.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        ui.data_mut(|data| data.insert_temp(id, (revision, pixels, texture.clone())));
        texture
    };
    if let Some(texture) = texture {
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        ui.painter()
            .image(texture.id(), rect, uv, egui::Color32::WHITE);
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No preview",
            egui::FontId::default(),
            ui.visuals().text_color(),
        );
    }
}
