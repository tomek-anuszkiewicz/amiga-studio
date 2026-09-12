//! Left Dock: CPU Registers Component
//!
//! Live D0-D7, A0-A7, PC, SR, CCR condition code LED toggles, interactive editing, and diff highlighting.

use egui::{Color32, RichText, Ui};
use m68000::{Cpu, CpuState};
use memory_bus::MemoryBus;

/// Register identifier for interactive inline editing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditRegister {
    D(usize),
    A(usize),
    PC,
    SR,
    USP,
    SSP,
}

pub fn render_registers(
    ui: &mut Ui,
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    prev_state: Option<&CpuState>,
    active_reg_edit: &mut Option<(EditRegister, String)>,
) {
    ui.heading("CPU Registers");

    let mut pc_to_set: Option<u32> = None;
    let state = &mut cpu.state;

    // Helper to pick text color based on whether value mutated since previous step
    let diff_color = |changed: bool| -> Color32 {
        if changed {
            Color32::from_rgb(0, 240, 255) // Vivid cyan highlight
        } else {
            Color32::from_rgb(220, 225, 235)
        }
    };

    // Helper to draw pill highlight background if value mutated
    let draw_diff_pill = |ui: &mut Ui, rect: egui::Rect, changed: bool| {
        if changed {
            ui.painter().rect_filled(
                rect.expand(2.0),
                3.0,
                Color32::from_rgba_unmultiplied(0, 180, 255, 55),
            );
        }
    };

    // 1. Data Registers D0-D7
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::CollapsingHeader::new(RichText::new("Data Registers (D0 - D7)").strong())
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("d_regs_grid")
                    .num_columns(6)
                    .spacing([8.0, 3.0])
                    .show(ui, |ui| {
                        for i in 0..8 {
                            let val = state.d_regs()[i];
                            let changed = prev_state.map_or(false, |p| p.d_regs()[i] != val);
                            let col = diff_color(changed);

                            ui.monospace(format!("D{}:", i))
                                .on_hover_text(format!("Data register D{} (32-bit)", i));

                            // Inline editing or interactive display
                            let is_editing = match active_reg_edit {
                                Some((EditRegister::D(reg_idx), _)) => *reg_idx == i,
                                _ => false,
                            };

                            if is_editing {
                                if let Some((_, buf)) = active_reg_edit {
                                    let enter_pressed =
                                        ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let esc_pressed =
                                        ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(buf)
                                            .desired_width(75.0)
                                            .font(egui::TextStyle::Monospace),
                                    );

                                    if esc_pressed || resp.clicked_elsewhere() {
                                        *active_reg_edit = None;
                                    } else if enter_pressed || resp.lost_focus() {
                                        let clean = buf.trim().trim_start_matches('$');
                                        if let Ok(new_val) = u32::from_str_radix(clean, 16) {
                                            state.set_d_long(i, new_val);
                                        }
                                        *active_reg_edit = None;
                                    }
                                }
                            } else {
                                let label_resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("${:08X}", val))
                                            .monospace()
                                            .color(col),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                draw_diff_pill(ui, label_resp.rect, changed);
                                if label_resp.clicked() {
                                    *active_reg_edit =
                                        Some((EditRegister::D(i), format!("{:08X}", val)));
                                }
                            }

                            let dec_str = format!("{:>8}", val as i32);
                            ui.add(egui::Label::new(
                                RichText::new(dec_str)
                                    .monospace()
                                    .color(Color32::from_rgb(130, 140, 155)),
                            ));

                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
    });

    // 2. Address Registers A0-A7
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::CollapsingHeader::new(RichText::new("Address Registers (A0 - A7)").strong())
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("a_regs_grid")
                    .num_columns(6)
                    .spacing([8.0, 3.0])
                    .show(ui, |ui| {
                        for i in 0..8 {
                            let val = state.a_regs()[i];
                            let changed = prev_state.map_or(false, |p| p.a_regs()[i] != val);
                            let col = diff_color(changed);

                            let is_sup = (state.sr & 0x2000) != 0;
                            let tooltip = if i == 7 {
                                if is_sup {
                                    "Address register A7 (Active Supervisor Stack Pointer - SSP)"
                                } else {
                                    "Address register A7 (Active User Stack Pointer - USP)"
                                }
                            } else {
                                "Address register (32-bit)"
                            };

                            ui.monospace(format!("A{}:", i)).on_hover_text(tooltip);

                            // Inline editing or interactive display
                            let is_editing = match active_reg_edit {
                                Some((EditRegister::A(reg_idx), _)) => *reg_idx == i,
                                _ => false,
                            };

                            if is_editing {
                                if let Some((_, buf)) = active_reg_edit {
                                    let enter_pressed =
                                        ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let esc_pressed =
                                        ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(buf)
                                            .desired_width(75.0)
                                            .font(egui::TextStyle::Monospace),
                                    );

                                    if esc_pressed || resp.clicked_elsewhere() {
                                        *active_reg_edit = None;
                                    } else if enter_pressed || resp.lost_focus() {
                                        let clean = buf.trim().trim_start_matches('$');
                                        if let Ok(new_val) = u32::from_str_radix(clean, 16) {
                                            state.set_a_long(i, new_val);
                                        }
                                        *active_reg_edit = None;
                                    }
                                }
                            } else {
                                let label_resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("${:08X}", val))
                                            .monospace()
                                            .color(col),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                draw_diff_pill(ui, label_resp.rect, changed);
                                if label_resp.clicked() {
                                    *active_reg_edit =
                                        Some((EditRegister::A(i), format!("{:08X}", val)));
                                }
                            }

                            let dec_str = format!("{:>8}", val as i32);
                            ui.add(egui::Label::new(
                                RichText::new(dec_str)
                                    .monospace()
                                    .color(Color32::from_rgb(130, 140, 155)),
                            ));

                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });

                // Inactive stack pointer & active status
                let is_supervisor = (state.sr & 0x2000) != 0;
                let (active_name, alt_label, alt_val, alt_id) = if is_supervisor {
                    ("SSP", "Inactive USP:", state.usp, EditRegister::USP)
                } else {
                    ("USP", "Inactive SSP:", state.ssp, EditRegister::SSP)
                };
                let alt_changed = prev_state.map_or(false, |p| {
                    if is_supervisor {
                        p.usp != state.usp
                    } else {
                        p.ssp != state.ssp
                    }
                });

                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 255),
                        format!("A7 = {active_name}"),
                    )
                    .on_hover_text(format!("A7 is currently bound to {active_name}"));
                    ui.label(RichText::new("|").color(Color32::from_rgb(80, 90, 105)));
                    ui.monospace(alt_label).on_hover_text(
                        "Inactive stack pointer for the alternate CPU privilege state",
                    );
                    let is_editing = match active_reg_edit {
                        Some((ref id, _)) => *id == alt_id,
                        None => false,
                    };

                    if is_editing {
                        if let Some((_, buf)) = active_reg_edit {
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let resp = ui.add(
                                egui::TextEdit::singleline(buf)
                                    .desired_width(75.0)
                                    .font(egui::TextStyle::Monospace),
                            );

                            if esc_pressed || resp.clicked_elsewhere() {
                                *active_reg_edit = None;
                            } else if enter_pressed || resp.lost_focus() {
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_val) = u32::from_str_radix(clean, 16) {
                                    if is_supervisor {
                                        state.usp = new_val;
                                    } else {
                                        state.ssp = new_val;
                                    }
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(format!("${:08X}", alt_val))
                                    .monospace()
                                    .color(diff_color(alt_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, resp.rect, alt_changed);
                        if resp.clicked() {
                            *active_reg_edit = Some((alt_id, format!("{:08X}", alt_val)));
                        }
                    }
                });
            });
    });

    // 3. Execution Status, PC & CCR (Compact 2-Column Layout)
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::CollapsingHeader::new(RichText::new("Execution Status & CCR").strong())
            .default_open(true)
            .show(ui, |ui| {
                let instruction_pc = state.instruction_pc & 0x00FF_FFFF;

                ui.columns(2, |cols| {
                    // Col 0: PC, SR, Mode, IPL
                    cols[0].vertical(|ui| {
                        ui.horizontal(|ui| {
                            let pc_changed = prev_state.map_or(false, |p| p.instruction_pc != state.instruction_pc);
                    ui.monospace("PC:")
                        .on_hover_ui(|ui| {
                            ui.heading("Program Counter (PC)");
                            ui.label(
                                format!(
                                    "Active instruction address: ${:06X}\n\
                                     Hardware prefetch bus PC: ${:06X}\n\n\
                                     The M68000 prefetch pipeline primes 2 instruction words (IR and IRC) \
                                     ahead from the bus, advancing the hardware bus register by 4 bytes.",
                                    instruction_pc, state.pc & 0x00FF_FFFF
                                ),
                            );
                        });

                    let is_editing_pc = matches!(active_reg_edit, Some((EditRegister::PC, _)));
                    if is_editing_pc {
                        if let Some((_, buf)) = active_reg_edit {
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let resp = ui.add(
                                egui::TextEdit::singleline(buf)
                                    .desired_width(70.0)
                                    .font(egui::TextStyle::Monospace),
                            );

                            if esc_pressed || resp.clicked_elsewhere() {
                                *active_reg_edit = None;
                            } else if enter_pressed || resp.lost_focus() {
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_pc) = u32::from_str_radix(clean, 16) {
                                    pc_to_set = Some(new_pc & 0x00FF_FFFF);
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let pc_resp = ui.add(
                            egui::Label::new(
                                RichText::new(format!("${:08X}", instruction_pc))
                                    .monospace()
                                    .color(diff_color(pc_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, pc_resp.rect, pc_changed);
                        if pc_resp.clicked() {
                            *active_reg_edit =
                                Some((EditRegister::PC, format!("{:08X}", instruction_pc)));
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let sr_changed = prev_state.map_or(false, |p| p.sr != state.sr);
                    ui.monospace("SR:")
                        .on_hover_ui(|ui| {
                            ui.heading("Status Register (SR)");
                            ui.label(
                                "16-bit register combining System Byte (bits 8-15) and Condition Code Register (CCR, bits 0-7).\n\
                                 • Bit 15: Trace mode (T)\n\
                                 • Bit 13: Supervisor mode (S)\n\
                                 • Bits 8-10: Interrupt Priority Level mask (IPL 0-7)",
                            );
                        });

                    let is_editing_sr = matches!(active_reg_edit, Some((EditRegister::SR, _)));
                    if is_editing_sr {
                        if let Some((_, buf)) = active_reg_edit {
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let resp = ui.add(
                                egui::TextEdit::singleline(buf)
                                    .desired_width(45.0)
                                    .font(egui::TextStyle::Monospace),
                            );

                            if esc_pressed || resp.clicked_elsewhere() {
                                *active_reg_edit = None;
                            } else if enter_pressed || resp.lost_focus() {
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_sr) = u16::from_str_radix(clean, 16) {
                                    state.sr = new_sr;
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let sr_resp = ui.add(
                            egui::Label::new(
                                RichText::new(format!("${:04X}", state.sr))
                                    .monospace()
                                    .color(diff_color(sr_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, sr_resp.rect, sr_changed);
                        if sr_resp.clicked() {
                            *active_reg_edit =
                                Some((EditRegister::SR, format!("{:04X}", state.sr)));
                        }
                    }
                });

                let is_supervisor = (state.sr & 0x2000) != 0;
                let mode_str = if is_supervisor {
                    "Supervisor [S]"
                } else {
                    "User [U]"
                };
                let mode_color = if is_supervisor {
                    Color32::from_rgb(255, 180, 50)
                } else {
                    Color32::from_rgb(100, 200, 255)
                };
                ui.colored_label(mode_color, mode_str);

                let ipl = (state.sr >> 8) & 0x07;
                ui.monospace(format!("IPL Mask: {}", ipl));
            });

            // Col 1: CCR Flags and Prefetch registers
            cols[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CCR:").strong())
                        .on_hover_text("Condition Code Register (CCR): Click any flag LED badge to toggle");

                    // Flags: X (bit 4), N (bit 3), Z (bit 2), V (bit 1), C (bit 0)
                    let flags = [
                        ("X", 0x10, "eXtend: Used for multi-precision arithmetic. Preserved by data movement."),
                        ("N", 0x08, "Negative: Set if result Most Significant Bit is 1 (negative)."),
                        ("Z", 0x04, "Zero: Set if result equals zero."),
                        ("V", 0x02, "oVerflow: Set if signed two's complement overflow occurred."),
                        ("C", 0x01, "Carry: Set if unsigned carry/borrow occurred during operation."),
                    ];

                    for (name, mask, desc) in flags {
                        let is_set = (state.sr & mask) != 0;
                        let flag_changed =
                            prev_state.map_or(false, |p| ((p.sr ^ state.sr) & mask) != 0);

                        let bg_color = if is_set {
                            Color32::from_rgb(34, 180, 80) // Bright Green
                        } else {
                            Color32::from_rgb(50, 55, 65) // Dark Gray
                        };
                        let text_color = if is_set {
                            Color32::BLACK
                        } else {
                            Color32::from_rgb(140, 145, 155)
                        };

                        let mut btn = egui::Button::new(RichText::new(name).color(text_color))
                            .fill(bg_color)
                            .min_size(egui::vec2(16.0, 18.0));

                        if flag_changed {
                            btn = btn
                                .stroke(egui::Stroke::new(1.5_f32, Color32::from_rgb(0, 240, 255)));
                        }

                        if ui.add(btn).on_hover_text(desc).clicked() {
                            state.sr ^= mask; // Toggle bit
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.monospace(format!("IR: ${:04X}", state.ir))
                        .on_hover_text("Instruction Register (IR): Current executing opcode");
                    ui.monospace(format!("IRC: ${:04X}", state.micro.irc))
                        .on_hover_text("Instruction Register Capture (IRC): Prefetched next pipeline word");
                });
            });
        });
            });
    });

    if let Some(target_pc) = pc_to_set {
        cpu.set_pc_and_prime_prefetch(target_pc, bus);
    }
}
