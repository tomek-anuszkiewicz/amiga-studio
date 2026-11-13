//! Binary loading and persistent file-error feedback.

use crate::EmulatorApp;

pub fn render(app: &mut EmulatorApp, ctx: &egui::Context) {
    if app.load_binary_modal_open {
        egui::Window::new("Load Binary into Memory")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_max_width(460.0);
                if let Some(source) = &app.pending_binary {
                    ui.label(format!("File: {}", source.name()));
                }
                ui.horizontal(|ui| {
                    ui.label("Target Address:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.load_target_addr_str)
                            .desired_width(80.0),
                    )
                    .on_hover_text("Hexadecimal memory address. Kickstart ROM starts at $FC0000 and is mirrored at $F80000.");
                });
                ui.label("Kickstart ROM images must be exactly 256 KiB (262144 bytes).");
                ui.checkbox(&mut app.load_auto_prime, "Auto-set PC & prime prefetch")
                    .on_hover_text("Start execution at the target address after a successful load.");
                if let Some(error) = &app.file_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Load into Memory")
                        .on_hover_text("Load the selected file. Invalid ROM sizes leave the machine unchanged.")
                        .clicked()
                    {
                        let clean = app.load_target_addr_str.trim().trim_start_matches('$').trim_start_matches("0x");
                        match u32::from_str_radix(clean, 16) {
                            Ok(target) => app.confirm_binary_load(target, app.load_auto_prime),
                            Err(_) => app.file_error = Some("Enter a valid hexadecimal target address.".to_string()),
                        }
                    }
                    if ui.button("Cancel").on_hover_text("Close without loading the file.").clicked() {
                        app.load_binary_modal_open = false;
                        app.pending_binary = None;
                        app.file_error = None;
                    }
                });
            });
    } else if let Some(error) = app.file_error.clone() {
        egui::Window::new("File loading failed")
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
