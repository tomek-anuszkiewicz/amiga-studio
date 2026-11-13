//! Left Dock: Emulation Engine & Bus Contention Status Card
//!
//! Live diagnostics for Chip RAM bus lock, DMA contention, IPL arbitration, and cycle metrics.

use crate::theme::ColorTokens;
use egui::{Grid, RichText, Ui};

#[derive(Debug, Default)]
pub(crate) struct EnginePresentation {
    chip_ram_blocked: bool,
    cck: String,
    instructions: String,
    ipl: String,
    interrupts_masked: bool,
    supervisor: bool,
}

impl EnginePresentation {
    pub(crate) fn refresh(&mut self, session: &debugger::DebuggerSession) {
        self.chip_ram_blocked = session.machine.physical_memory.chip_ram_blocked;
        self.cck = format!("#{}", session.machine.cck);
        self.instructions = session.instructions_executed.to_string();
        let state = &session.machine.cpu.state;
        self.ipl = format!("Level {} (Bits 8-10)", state.interrupt_mask());
        self.interrupts_masked =
            u16::from(state.interrupt_mask()) == cpu::state::MASK_SR_INTERRUPT_LEVEL;
        self.supervisor = state.is_supervisor();
    }
}

pub(crate) fn render_engine_status(ui: &mut Ui, tokens: &ColorTokens, data: &EnginePresentation) {
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
                        if data.chip_ram_blocked {
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
                        ui.monospace(RichText::new(&data.cck).color(tokens.text_monospace));
                        ui.end_row();

                        // 3. Instruction Counter
                        ui.label(
                            RichText::new("Instructions Retired:").color(tokens.text_secondary),
                        )
                        .on_hover_text("Total instructions executed since machine start/reset");
                        ui.monospace(
                            RichText::new(&data.instructions).color(tokens.text_monospace),
                        );
                        ui.end_row();

                        // 4. Interrupt Priority Level (IPL)
                        ui.label(
                            RichText::new("Interrupt Mask (IPL):").color(tokens.text_secondary),
                        )
                        .on_hover_text("Current CPU interrupt priority mask (0-7)");
                        let ipl_color = if data.interrupts_masked {
                            tokens.accent_warning // Mask 7 disables maskable interrupts
                        } else {
                            tokens.accent_pc
                        };
                        ui.monospace(RichText::new(&data.ipl).color(ipl_color));
                        ui.end_row();

                        // 5. Privilege State
                        ui.label(RichText::new("Privilege Mode:").color(tokens.text_secondary));
                        if data.supervisor {
                            ui.colored_label(tokens.accent_warning, "Supervisor Mode (SSP)");
                        } else {
                            ui.colored_label(tokens.accent_pc, "User Mode (USP)");
                        }
                        ui.end_row();
                    });
            });
    });
}
