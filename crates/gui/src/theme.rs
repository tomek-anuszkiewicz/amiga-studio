//! Amiga 500 Theme Engine & Palettes
//!
//! Provides Dark (default), Light, and Classic Amiga Workbench color schemes.

pub mod tokens;
pub use tokens::ColorTokens;

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
    /// Returns the semantic color tokens for this theme
    pub fn tokens(&self) -> &'static ColorTokens {
        match self {
            Self::Dark => &ColorTokens::DARK,
            Self::Light => &ColorTokens::LIGHT,
            Self::ClassicWorkbench => &ColorTokens::CLASSIC_WORKBENCH,
        }
    }

    /// Applies the theme palette to the active egui Context
    pub fn apply(&self, ctx: &egui::Context) {
        let tokens = self.tokens();
        match self {
            Self::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.window_fill = tokens.canvas_bg;
                visuals.panel_fill = tokens.panel_bg;
                visuals.selection.bg_fill = tokens.accent_pc_bg;
                visuals.selection.stroke = egui::Stroke::new(1.0_f32, tokens.accent_pc);
                visuals.widgets.noninteractive.bg_fill = tokens.card_bg;
                visuals.widgets.noninteractive.fg_stroke =
                    egui::Stroke::new(1.0_f32, tokens.text_secondary);
                visuals.widgets.inactive.bg_fill = tokens.panel_bg;
                visuals.widgets.inactive.fg_stroke =
                    egui::Stroke::new(1.0_f32, tokens.text_primary);
                visuals.widgets.hovered.bg_fill = tokens.card_bg;
                visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, tokens.accent_pc);
                visuals.widgets.active.bg_fill = tokens.accent_pc_bg;
                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, tokens.text_primary);
                ctx.set_visuals(visuals);
            }
            Self::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.window_fill = tokens.canvas_bg;
                visuals.panel_fill = tokens.panel_bg;
                visuals.selection.bg_fill = tokens.accent_pc_bg;
                visuals.selection.stroke = egui::Stroke::new(1.0_f32, tokens.accent_pc);
                visuals.widgets.noninteractive.bg_fill = tokens.card_bg;
                visuals.widgets.inactive.bg_fill = tokens.panel_bg;
                ctx.set_visuals(visuals);
            }
            Self::ClassicWorkbench => {
                let mut visuals = egui::Visuals::dark();
                visuals.window_fill = tokens.canvas_bg;
                visuals.panel_fill = tokens.panel_bg;
                visuals.selection.bg_fill = tokens.accent_pc;
                visuals.widgets.noninteractive.bg_fill = tokens.card_bg;
                visuals.widgets.noninteractive.fg_stroke =
                    egui::Stroke::new(1.0_f32, tokens.text_primary);
                visuals.widgets.inactive.bg_fill = tokens.canvas_bg;
                visuals.widgets.inactive.fg_stroke =
                    egui::Stroke::new(1.0_f32, tokens.text_primary);
                visuals.widgets.hovered.bg_fill = tokens.accent_pc;
                visuals.widgets.hovered.fg_stroke =
                    egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
                visuals.widgets.active.bg_fill = tokens.accent_pc;
                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
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
