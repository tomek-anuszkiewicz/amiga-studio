//! Amiga 500 Theme Engine & Palettes
//!
//! Provides Dark (default), Light, and Classic Amiga Workbench color schemes.

use serde::{Deserialize, Serialize};

/// Supported GUI visual themes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    ClassicWorkbench,
}

impl AppTheme {
    /// Applies the theme palette to the active egui Context
    pub fn apply(&self, ctx: &egui::Context) {
        match self {
            Self::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.window_fill = egui::Color32::from_rgb(24, 26, 32);
                visuals.panel_fill = egui::Color32::from_rgb(18, 20, 24);
                visuals.selection.bg_fill = egui::Color32::from_rgb(20, 70, 110); // Deep navy/cyan with high contrast
                visuals.selection.stroke =
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 210, 255));
                visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 29, 36);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 38, 48);
                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 54, 68);
                visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 70, 95);
                ctx.set_visuals(visuals);
            }
            Self::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.window_fill = egui::Color32::from_rgb(245, 246, 248);
                visuals.panel_fill = egui::Color32::from_rgb(238, 240, 244);
                visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(230, 232, 238);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(220, 224, 232);
                ctx.set_visuals(visuals);
            }
            Self::ClassicWorkbench => {
                let mut visuals = egui::Visuals::dark();
                // Authentic Amiga Workbench 1.3 palette
                // Deep Blue background (#0055AA), Topaz Orange accents (#FFAA00 / #FF8800)
                let wb_blue = egui::Color32::from_rgb(0, 85, 170);
                let wb_dark_blue = egui::Color32::from_rgb(0, 60, 125);
                let wb_orange = egui::Color32::from_rgb(255, 136, 0);
                let wb_white = egui::Color32::from_rgb(255, 255, 255);
                let wb_black = egui::Color32::from_rgb(0, 0, 0);

                visuals.window_fill = wb_dark_blue;
                visuals.panel_fill = wb_blue;
                visuals.selection.bg_fill = wb_orange;
                visuals.widgets.noninteractive.bg_fill = wb_dark_blue;
                visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, wb_white);
                visuals.widgets.inactive.bg_fill = wb_dark_blue;
                visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, wb_white);
                visuals.widgets.hovered.bg_fill = wb_orange;
                visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, wb_black);
                visuals.widgets.active.bg_fill = wb_orange;
                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, wb_black);
                ctx.set_visuals(visuals);
            }
        }

        // Global Scrollbar Styling (comfortable 12px width & high dormant visibility)
        let mut style = (*ctx.style()).clone();
        style.spacing.scroll.bar_width = 12.0;
        style.spacing.scroll.dormant_background_opacity = 0.35;
        style.spacing.scroll.dormant_handle_opacity = 0.65;
        style.spacing.scroll.active_background_opacity = 0.60;
        style.spacing.scroll.active_handle_opacity = 1.0;
        ctx.set_style(style);
    }
}
