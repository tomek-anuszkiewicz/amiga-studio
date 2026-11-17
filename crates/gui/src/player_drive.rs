//! Brief player indicators for drive events observed between host publications.

use crate::theme::ColorTokens;

const ACTIVITY_SECONDS: f64 = 0.120;
const INDICATOR_RADIUS_RATIO: f32 = 0.22;
const ACTIVE_LABELS: [&str; config::FLOPPY_DRIVE_COUNT] = [
    "DF0 activity: active",
    "DF1 activity: active",
    "DF2 activity: active",
    "DF3 activity: active",
];
const IDLE_LABELS: [&str; config::FLOPPY_DRIVE_COUNT] = [
    "DF0 activity: idle",
    "DF1 activity: idle",
    "DF2 activity: idle",
    "DF3 activity: idle",
];

#[derive(Debug, Default)]
pub struct DriveActivity {
    generation: Option<(u64, u64)>,
    sequence: u64,
    until: f64,
}

impl DriveActivity {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        drive: usize,
        generation: (u64, u64),
        sequence: u64,
        tokens: &ColorTokens,
    ) {
        let now = ui.input(|input| input.time);
        if self.generation != Some(generation) {
            // A restored machine is a new baseline, not new disk activity.
            self.generation = Some(generation);
            self.until = now;
        } else if self.sequence != sequence {
            self.until = now + ACTIVITY_SECONDS;
        }
        self.sequence = sequence;
        let active = now < self.until;
        if active {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(self.until - now));
        }
        let height = ui.text_style_height(&egui::TextStyle::Body);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(height, height), egui::Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                if active {
                    ACTIVE_LABELS[drive]
                } else {
                    IDLE_LABELS[drive]
                },
            )
        });
        response.on_hover_text("Disk activity: head steps or controller data transfers. Motor rotation alone does not light this indicator.");
        ui.painter().circle_filled(
            rect.center(),
            height * INDICATOR_RADIUS_RATIO,
            if active {
                tokens.accent_success
            } else {
                tokens.border_subtle
            },
        );
    }
}
