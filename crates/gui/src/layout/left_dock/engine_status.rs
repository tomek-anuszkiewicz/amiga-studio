//! Left Dock: Emulation Engine & Bus Contention Status Card
//!
//! Live diagnostics for Chip RAM bus lock, DMA contention, IPL arbitration, and cycle metrics.

use crate::theme::ColorTokens;
use egui::{Grid, RichText, Ui};

pub(crate) fn render_engine_status(
    ui: &mut Ui,
    tokens: &ColorTokens,
    chip_ram_blocked: bool,
    instructions_executed: u64,
    current_cck: u64,
    sr: u16,
) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::CollapsingHeader::new(RichText::new("Emulation Engine Status").strong())
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("engine_status_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        // 1. Chip RAM Bus Arbitration State
                        ui.label(RichText::new("Chip RAM Bus:").color(tokens.text_secondary))
                            .on_hover_text("Bus arbitration state for Agnus, Paula, and CPU");
                        if chip_ram_blocked {
                            ui.colored_label(tokens.accent_warning, "LOCKED (DMA Wait State)")
                                .on_hover_text(
                                    "Chip RAM is currently locked by custom chip DMA channel",
                                );
                        } else {
                            ui.colored_label(tokens.accent_success, "FREE (CPU Available)")
                                .on_hover_text(
                                    "Chip RAM is unlocked and available for CPU memory accesses",
                                );
                        }
                        ui.end_row();

                        // 2. Execution Clock & Timing
                        ui.label(RichText::new("Color Clock (CCK):").color(tokens.text_secondary))
                            .on_hover_text("Amiga 3.54 MHz Color Clock master counter");
                        ui.monospace(
                            RichText::new(format!("#{current_cck}")).color(tokens.text_monospace),
                        );
                        ui.end_row();

                        // 3. Instruction Counter
                        ui.label(
                            RichText::new("Instructions Retired:").color(tokens.text_secondary),
                        )
                        .on_hover_text("Total instructions executed since machine start/reset");
                        ui.monospace(
                            RichText::new(format!("{instructions_executed}"))
                                .color(tokens.text_monospace),
                        );
                        ui.end_row();

                        // 4. Interrupt Priority Level (IPL)
                        let ipl_mask = (sr >> 8) & 0x07;
                        ui.label(
                            RichText::new("Interrupt Mask (IPL):").color(tokens.text_secondary),
                        )
                        .on_hover_text("Current CPU interrupt priority mask (0-7)");
                        let ipl_color = if ipl_mask == 7 {
                            tokens.accent_warning // Mask 7 disables maskable interrupts
                        } else {
                            tokens.accent_pc
                        };
                        ui.monospace(
                            RichText::new(format!("Level {ipl_mask} (Bits 8-10)")).color(ipl_color),
                        );
                        ui.end_row();

                        // 5. Privilege State
                        let is_sup = (sr & 0x2000) != 0;
                        ui.label(RichText::new("Privilege Mode:").color(tokens.text_secondary));
                        if is_sup {
                            ui.colored_label(tokens.accent_warning, "Supervisor Mode (SSP)");
                        } else {
                            ui.colored_label(tokens.accent_pc, "User Mode (USP)");
                        }
                        ui.end_row();
                    });
            });
    });
}
