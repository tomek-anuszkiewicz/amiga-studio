//! Recent ROM selection and persistent file-error feedback.

use crate::EmulatorApp;

#[cfg(target_arch = "wasm32")]
pub(super) fn render_recent_files(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    ui.separator();
    ui.label("Recent ROMs");
    let names = app.browser_history.names();
    if names.is_empty() {
        ui.add_enabled(false, egui::Button::new("No recent ROMs"));
    }
    for (index, name) in names.iter().enumerate() {
        if ui.push_id(("rom", index), |ui| {
            ui.button(name).on_hover_text("Choose a file from this file's folder. Browsers without folder support use their default location.").clicked()
        }).inner {
            app.open_recent_browser_file(index);
            ui.close_kind(egui::UiKind::Menu);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn render_recent_files(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    ui.separator();
    ui.label("Recent ROMs");
    let mut selected_rom = None;
    for (index, path) in app.file_history.roms.iter().enumerate() {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if ui
            .push_id(("rom", index), |ui| {
                ui.button(name.as_ref())
                    .on_hover_ui(|ui| {
                        ui.label(path.display().to_string());
                        ui.label("Reload this ROM and reset the machine. Press Run to boot.");
                    })
                    .clicked()
            })
            .inner
        {
            selected_rom = Some(path.clone());
        }
    }
    if app.file_history.roms.is_empty() {
        ui.add_enabled(false, egui::Button::new("No recent ROMs"));
    }
    if let Some(path) = selected_rom {
        app.load_rom_file(&path);
        ui.close_kind(egui::UiKind::Menu);
    }
}

const ERROR_DIALOG: &str = "file-error-dialog";

pub(crate) fn limit_dialog_focus(app: &EmulatorApp, ctx: &egui::Context) {
    if app.file_error.is_none() {
        return;
    }
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique(ERROR_DIALOG));
    let focused = ctx.memory(|memory| memory.focused());
    let outside = focused.filter(|id| {
        ctx.read_response(*id)
            .is_none_or(|response| response.layer_id != layer)
    });
    ctx.memory_mut(|memory| {
        memory.set_modal_layer(layer);
        if let Some(id) = outside {
            memory.surrender_focus(id);
        }
    });
}

pub fn render(app: &mut EmulatorApp, ctx: &egui::Context) {
    if let Some(error) = app.file_error.clone() {
        let title = if app.host.error.is_some() {
            "Emulation stopped"
        } else {
            "File operation failed"
        };
        egui::Window::new(title)
            .id(egui::Id::unique(ERROR_DIALOG))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.colored_label(ui.visuals().error_fg_color, error);
                if ui
                    .button("Close")
                    .on_hover_text("Dismiss this error.")
                    .clicked()
                {
                    app.file_error = None;
                }
            });
    }
}
