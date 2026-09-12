//! Left Dock: Microcode State Inspector Component
//!
//! Visualizes micro-step state machine, CCK phases, staging registers, and bus contention.

use egui::{Color32, RichText, Ui};
use m68000::CpuState;

pub fn render_microcode(ui: &mut Ui, state: &CpuState, bus_blocked: bool) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());

        egui::CollapsingHeader::new(RichText::new("Microcode Inspector").strong())
            .default_open(true)
            .show(ui, |ui| {
                // Tooltip on inspector header
                ui.label(
                    RichText::new("M68000 micro-operation state machine & cycle timing")
                        .small()
                        .color(Color32::from_rgb(150, 160, 175)),
                )
                .on_hover_ui(|ui| {
                    ui.heading("M68000 Microcode State Machine");
                    ui.label(
                        "The M68000 executes complex CISC instructions through a sequence of \
                         atomic 2-clock and 4-clock micro-steps rather than monolithic execution. \
                         Each micro-step models physical bus transfers, ALU operations, and \
                         pipeline progression synchronized to Amiga Color Clock (CCK) phases.",
                    );
                });

                // 1. Active Opcode, Micro-step progress & Phase
                ui.horizontal(|ui| {
                    ui.monospace(format!("IR: ${:04X}", state.ir));

                    let step_idx = state.micro.micro_step;
                    let total_steps = state.micro.current_steps.len();
                    ui.monospace("Step:");
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 180),
                        format!("{}/{}", step_idx, total_steps.max(1)),
                    );

                    let fraction = if total_steps > 0 {
                        (step_idx as f32) / (total_steps as f32)
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(fraction).desired_width(40.0));

                    let is_cck1 = (state.cycle_counter & 2) == 0;
                    let phase_label = if is_cck1 {
                        ui.colored_label(Color32::from_rgb(100, 200, 255), "[CCK1]")
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 180, 50), "[CCK2]")
                    };

                    phase_label.on_hover_ui(|ui| {
                        ui.heading("Color Clock (CCK) Phases");
                        ui.label(
                            "The Amiga system clock (~7.09 MHz PAL) divides into Color Clocks (CCK, ~3.54 MHz).\n\
                             1 M68000 bus cycle = 4 CPU clocks = 2 CCK cycles (CCK1 + CCK2).\n\n\
                             • CCK1 (Drive): Address bus drive and /AS assertion.\n\
                             • CCK2 (Latch/ALU): Bus data latching, /DTACK handshake, and CCR flag evaluation.",
                        );
                    });
                });

                // 2. Real-Time Remaining Clock & Cycle Counters
                let active_rem = state.micro.clocks_remaining;
                let mut upcoming_clocks = 0u32;
                let cur_step = state.micro.micro_step as usize;
                if cur_step + 1 < state.micro.current_steps.len() {
                    for s in &state.micro.current_steps[cur_step + 1..] {
                        upcoming_clocks += s.base_clocks as u32;
                    }
                }
                let total_rem_clocks = (active_rem as u32) + upcoming_clocks;
                let total_rem_cck = (total_rem_clocks + 1) / 2;

                ui.horizontal(|ui| {
                    ui.monospace(
                        RichText::new(format!(
                            "Step rem: {} clk | Total rem: {} clk ({} CCK)",
                            active_rem, total_rem_clocks, total_rem_cck
                        ))
                        .color(Color32::from_rgb(180, 210, 240))
                        .small(),
                    )
                    .on_hover_ui(|ui| {
                        ui.label(
                            "Real-time clock counter showing how many host/bus clock cycles remain \
                             before the active micro-step finishes and how many total clocks remain \
                             until the entire instruction retires.",
                        );
                    });
                });

                ui.separator();

                // 3. Dual Staging, EA Registers & Bus Contention (Compact 4-Column Grid)
                egui::Grid::new("staging_regs_compact_grid")
                    .num_columns(4)
                    .spacing([6.0, 2.0])
                    .show(ui, |ui| {
                        ui.monospace("addr1 (X1):")
                            .on_hover_text("Staged source address");
                        ui.monospace(format!("${:08X}", state.micro.addr1))
                            .on_hover_text("Staged source address");
                        ui.monospace("addr2 (X2):")
                            .on_hover_text("Staged destination address");
                        ui.monospace(format!("${:08X}", state.micro.addr2))
                            .on_hover_text("Staged destination address");
                        ui.end_row();

                        ui.monospace("source:")
                            .on_hover_text("Source operand value");
                        ui.monospace(format!("${:08X}", state.micro.source))
                            .on_hover_text("Source operand value");
                        ui.monospace("destination:")
                            .on_hover_text("Destination operand value");
                        ui.monospace(format!("${:08X}", state.micro.destination))
                            .on_hover_text("Destination operand value");
                        ui.end_row();

                        ui.monospace("ea_addr:")
                            .on_hover_text("Effective address");
                        ui.monospace(format!("${:08X}", state.micro.ea_addr))
                            .on_hover_text("Effective address");
                        ui.monospace("Chip RAM:")
                            .on_hover_text("Chip RAM DMA arbitration: Agnus/Denise bus locking status");
                        if bus_blocked {
                            ui.colored_label(Color32::from_rgb(255, 70, 70), "BLOCKED")
                                .on_hover_text("Chip RAM is currently locked by custom chip DMA; CPU is inserting wait states");
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(70, 220, 70),
                                format!("FREE ({}c)", state.micro.clocks_remaining),
                            )
                            .on_hover_text("Chip RAM is accessible to CPU");
                        }
                        ui.end_row();
                    });
            });
    });
}
