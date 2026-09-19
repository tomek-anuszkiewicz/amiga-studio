//! Semantic Color Tokens for Amiga Developer Studio
//!
//! Provides consistent, eye-friendly, and accessible colors across all panels.

use egui::Color32;

/// Centralized semantic color tokens for surfaces, text, badges, and accents
#[derive(Debug, Clone, Copy)]
pub struct ColorTokens {
    // Surfaces
    pub canvas_bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub border_subtle: Color32,
    pub border_active: Color32,

    // Typography
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_monospace: Color32,

    // Functional & Semantic Accents
    pub accent_pc: Color32,
    pub accent_pc_bg: Color32,
    pub accent_diff: Color32,
    pub accent_diff_bg: Color32,
    pub accent_success: Color32,
    pub accent_warning: Color32,
    pub accent_error: Color32,
    pub address_blue: Color32,

    // Condition Code Register (CCR) LED Badges
    pub ccr_active_bg: Color32,
    pub ccr_active_text: Color32,
    pub ccr_inactive_bg: Color32,
    pub ccr_inactive_text: Color32,
}

impl ColorTokens {
    /// Retro Studio Pro Dark palette (default)
    pub(crate) const DARK: Self = Self {
        canvas_bg: Color32::from_rgb(14, 17, 23),
        panel_bg: Color32::from_rgb(22, 27, 34),
        card_bg: Color32::from_rgb(30, 36, 48),
        border_subtle: Color32::from_rgb(46, 54, 70),
        border_active: Color32::from_rgb(56, 189, 248),

        text_primary: Color32::from_rgb(226, 232, 240),
        text_secondary: Color32::from_rgb(148, 163, 184),
        text_muted: Color32::from_rgb(100, 116, 139),
        text_monospace: Color32::from_rgb(241, 245, 249),

        accent_pc: Color32::from_rgb(56, 189, 248),
        accent_pc_bg: Color32::from_rgba_premultiplied(12, 74, 110, 140),
        accent_diff: Color32::from_rgb(103, 232, 249),
        accent_diff_bg: Color32::from_rgba_premultiplied(8, 51, 68, 120),
        accent_success: Color32::from_rgb(52, 211, 153),
        accent_warning: Color32::from_rgb(251, 191, 36),
        accent_error: Color32::from_rgb(248, 113, 113),
        address_blue: Color32::from_rgb(129, 140, 248),

        ccr_active_bg: Color32::from_rgb(5, 150, 105),
        ccr_active_text: Color32::WHITE,
        ccr_inactive_bg: Color32::from_rgb(42, 50, 65),
        ccr_inactive_text: Color32::from_rgb(226, 232, 240),
    };

    /// Clean Light palette
    pub(crate) const LIGHT: Self = Self {
        canvas_bg: Color32::from_rgb(245, 247, 250),
        panel_bg: Color32::from_rgb(238, 242, 246),
        card_bg: Color32::from_rgb(228, 233, 240),
        border_subtle: Color32::from_rgb(203, 213, 225),
        border_active: Color32::from_rgb(2, 132, 199),

        text_primary: Color32::from_rgb(15, 23, 42),
        text_secondary: Color32::from_rgb(71, 85, 105),
        text_muted: Color32::from_rgb(148, 163, 184),
        text_monospace: Color32::from_rgb(15, 23, 42),

        accent_pc: Color32::from_rgb(2, 132, 199),
        accent_pc_bg: Color32::from_rgba_premultiplied(186, 230, 253, 160),
        accent_diff: Color32::from_rgb(8, 145, 178),
        accent_diff_bg: Color32::from_rgba_premultiplied(207, 250, 254, 160),
        accent_success: Color32::from_rgb(16, 185, 129),
        accent_warning: Color32::from_rgb(217, 119, 6),
        accent_error: Color32::from_rgb(225, 29, 72),
        address_blue: Color32::from_rgb(79, 70, 229),

        ccr_active_bg: Color32::from_rgb(16, 185, 129),
        ccr_active_text: Color32::WHITE,
        ccr_inactive_bg: Color32::from_rgb(203, 213, 225),
        ccr_inactive_text: Color32::from_rgb(51, 65, 85),
    };

    /// Classic Amiga Workbench 1.3 palette
    pub(crate) const CLASSIC_WORKBENCH: Self = Self {
        canvas_bg: Color32::from_rgb(0, 60, 125),
        panel_bg: Color32::from_rgb(0, 85, 170),
        card_bg: Color32::from_rgb(0, 70, 140),
        border_subtle: Color32::from_rgb(0, 110, 215),
        border_active: Color32::from_rgb(255, 136, 0),

        text_primary: Color32::WHITE,
        text_secondary: Color32::from_rgb(220, 235, 255),
        text_muted: Color32::from_rgb(120, 170, 230),
        text_monospace: Color32::WHITE,

        accent_pc: Color32::from_rgb(255, 170, 0),
        accent_pc_bg: Color32::from_rgba_premultiplied(255, 136, 0, 120),
        accent_diff: Color32::from_rgb(255, 200, 50),
        accent_diff_bg: Color32::from_rgba_premultiplied(255, 170, 0, 90),
        accent_success: Color32::from_rgb(80, 220, 100),
        accent_warning: Color32::from_rgb(255, 136, 0),
        accent_error: Color32::from_rgb(255, 80, 80),
        address_blue: Color32::from_rgb(140, 200, 255),

        ccr_active_bg: Color32::from_rgb(255, 136, 0),
        ccr_active_text: Color32::BLACK,
        ccr_inactive_bg: Color32::from_rgb(0, 50, 100),
        ccr_inactive_text: Color32::WHITE,
    };
}
