//! Left Dock: Microcode State Inspector Component
//!
//! Visualizes micro-step state machine, CCK phases, staging registers, and bus contention.

use cpu::CpuState;
use egui::{Color32, RichText, Ui};

#[derive(Debug, Default)]
pub(crate) struct MicrocodePresentation {
    ir: String,
    step: String,
    fraction: f32,
    cck1: bool,
    remaining: String,
    staging: [String; 5],
    bus_blocked: bool,
    bus_free: String,
}

impl MicrocodePresentation {
    pub(crate) fn refresh(&mut self, state: &CpuState, bus_blocked: bool) {
        let micro = state.micro();
        let total_steps = micro.current_steps.len();
        self.ir = format!("IR: ${:04X}", state.ir());
        self.step = format!("{}/{}", micro.micro_step, total_steps.max(1));
        self.fraction = if total_steps > 0 {
            micro.micro_step as f32 / total_steps as f32
        } else {
            0.0
        };
        self.cck1 = (state.cycle_counter() & u64::from(cpu::MicroStep::CCK_CLOCKS)) == 0;
        let upcoming: u32 = micro
            .current_steps
            .iter()
            .skip(micro.micro_step as usize + 1)
            .map(|s| u32::from(s.base_clocks))
            .sum();
        let total = u32::from(micro.clocks_remaining) + upcoming;
        self.remaining = format!(
            "Step rem: {} clk | Total rem: {} clk ({} CCK)",
            micro.clocks_remaining,
            total,
            (total + 1) / u32::from(cpu::MicroStep::CCK_CLOCKS)
        );
        self.staging = [
            micro.addr1,
            micro.addr2,
            micro.source,
            micro.destination,
            micro.ea_addr,
        ]
        .map(|v| format!("${v:08X}"));
        self.bus_blocked = bus_blocked;
        self.bus_free = format!("FREE ({}c)", micro.clocks_remaining);
    }
}

pub(crate) fn render_microcode(ui: &mut Ui, data: &MicrocodePresentation) {
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
                    ui.monospace(&data.ir);

                    ui.monospace("Step:");
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 180),
                        &data.step,
                    );

                    ui.add(egui::ProgressBar::new(data.fraction).desired_width(40.0));

                    let phase_label = if data.cck1 {
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
                ui.horizontal(|ui| {
                    ui.monospace(
                        RichText::new(&data.remaining)
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
                        ui.monospace(&data.staging[0])
                            .on_hover_text("Staged source address");
                        ui.monospace("addr2 (X2):")
                            .on_hover_text("Staged destination address");
                        ui.monospace(&data.staging[1])
                            .on_hover_text("Staged destination address");
                        ui.end_row();

                        ui.monospace("source:")
                            .on_hover_text("Source operand value");
                        ui.monospace(&data.staging[2])
                            .on_hover_text("Source operand value");
                        ui.monospace("destination:")
                            .on_hover_text("Destination operand value");
                        ui.monospace(&data.staging[3])
                            .on_hover_text("Destination operand value");
                        ui.end_row();

                        ui.monospace("ea_addr:")
                            .on_hover_text("Effective address");
                        ui.monospace(&data.staging[4])
                            .on_hover_text("Effective address");
                        ui.monospace("Chip RAM:")
                            .on_hover_text("Chip RAM DMA arbitration: Agnus/Denise bus locking status");
                        if data.bus_blocked {
                            ui.colored_label(Color32::from_rgb(255, 70, 70), "BLOCKED")
                                .on_hover_text("Chip RAM is currently locked by custom chip DMA; CPU is inserting wait states");
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(70, 220, 70),
                                &data.bus_free,
                            )
                            .on_hover_text("Chip RAM is accessible to CPU");
                        }
                        ui.end_row();
                    });
            });
    });
}
