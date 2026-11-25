//! Persistent feedback for failed file operations.

use crate::EmulatorApp;

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
