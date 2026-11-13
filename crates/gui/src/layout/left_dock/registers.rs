//! Left Dock: CPU Registers Component
//!
//! Live D0-D7, A0-A7, PC, SR, CCR condition code LED toggles, interactive editing, and diff highlighting.

use crate::theme::ColorTokens;
use cpu::{Cpu, CpuState};
use egui::{Color32, RichText, Ui};
use physical_memory::PhysicalMemory;

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

const D_LABELS: [&str; 8] = ["D0:", "D1:", "D2:", "D3:", "D4:", "D5:", "D6:", "D7:"];
const D_TOOLTIPS: [&str; 8] = [
    "Data register D0 (32-bit general data/scratch register)",
    "Data register D1 (32-bit general data/scratch register)",
    "Data register D2 (32-bit general data/scratch register)",
    "Data register D3 (32-bit general data/scratch register)",
    "Data register D4 (32-bit general data/scratch register)",
    "Data register D5 (32-bit general data/scratch register)",
    "Data register D6 (32-bit general data/scratch register)",
    "Data register D7 (32-bit general data/scratch register)",
];
const A_LABELS: [&str; 8] = ["A0:", "A1:", "A2:", "A3:", "A4:", "A5:", "A6:", "A7:"];

#[derive(Debug, Default)]
pub(crate) struct RegisterValue {
    value: u32,
    changed: bool,
    hex: String,
    decimal: String,
}

impl RegisterValue {
    fn new(value: u32, previous: Option<u32>, digits: usize) -> Self {
        Self {
            value,
            changed: previous.is_some_and(|old| old != value),
            hex: format!("${value:0digits$X}"),
            decimal: format!("{:>11}", value as i32),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct RegisterPresentation {
    d: [RegisterValue; cpu::state::REGISTER_COUNT],
    a: [RegisterValue; cpu::state::REGISTER_COUNT],
    pc: RegisterValue,
    sr: RegisterValue,
    alt: RegisterValue,
    supervisor: bool,
    previous_sr: u16,
    ir: String,
    irc: String,
    ipl: String,
    pc_tooltip: String,
}

impl RegisterPresentation {
    fn refresh(&mut self, state: &CpuState, previous: Option<&CpuState>) {
        self.d = std::array::from_fn(|i| {
            RegisterValue::new(state.d_long(i), previous.map(|p| p.d_long(i)), 8)
        });
        self.a = std::array::from_fn(|i| {
            RegisterValue::new(state.a_long(i), previous.map(|p| p.a_long(i)), 8)
        });
        self.pc = RegisterValue::new(
            state.instruction_pc() & physical_memory::MASK_PHYSICAL_ADDRESS,
            previous.map(CpuState::instruction_pc),
            8,
        );
        self.sr = RegisterValue::new(
            u32::from(state.sr()),
            previous.map(|p| u32::from(p.sr())),
            4,
        );
        self.supervisor = state.is_supervisor();
        self.alt = if self.supervisor {
            RegisterValue::new(state.usp(), previous.map(CpuState::usp), 8)
        } else {
            RegisterValue::new(state.ssp(), previous.map(CpuState::ssp), 8)
        };
        self.previous_sr = previous.map_or_else(|| state.sr(), CpuState::sr);
        self.ir = format!("IR: ${:04X}", state.ir());
        self.irc = format!("IRC: ${:04X}", state.micro().irc);
        self.ipl = format!("IPL Mask: {}", state.interrupt_mask());
        self.pc_tooltip = format!("Active instruction address: ${:06X}\nHardware prefetch bus PC: ${:06X}\n\nThe M68000 prefetch pipeline primes 2 instruction words (IR and IRC) ahead from the bus, advancing the hardware bus register by 4 bytes.", self.pc.value, state.pc() & physical_memory::MASK_PHYSICAL_ADDRESS);
    }
}

pub(crate) fn render_registers(
    ui: &mut Ui,
    presentation: &mut RegisterPresentation,
    refresh: bool,
    interactive: bool,
    tokens: &ColorTokens,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
    prev_state: Option<&CpuState>,
    active_reg_edit: &mut Option<(EditRegister, String)>,
) -> bool {
    if refresh {
        presentation.refresh(&cpu.state, prev_state);
    }
    ui.heading("CPU Registers");

    let mut pc_to_set: Option<u32> = None;
    let state = &mut cpu.state;

    // Helper to pick text color based on whether value mutated since previous step
    let diff_color = |changed: bool| -> Color32 {
        if changed {
            tokens.accent_diff
        } else {
            tokens.text_primary
        }
    };

    // Helper to draw pill highlight background if value mutated
    let draw_diff_pill = |ui: &mut Ui, rect: egui::Rect, changed: bool| {
        if changed {
            ui.painter()
                .rect_filled(rect.expand(2.0), 3.0, tokens.accent_diff_bg);
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
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for i in 0..cpu::state::REGISTER_COUNT {
                            let val = presentation.d[i].value;
                            let changed = presentation.d[i].changed;
                            let col = diff_color(changed);

                            ui.monospace(D_LABELS[i]).on_hover_text(D_TOOLTIPS[i]);

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
                                    let resp = ui
                                        .push_id("edit_d", |ui| {
                                            let r = ui.add(
                                                egui::TextEdit::singleline(buf)
                                                    .desired_width(75.0)
                                                    .font(egui::TextStyle::Monospace),
                                            );
                                            if interactive {
                                                r.request_focus();
                                            }
                                            r
                                        })
                                        .inner;

                                    if interactive && esc_pressed {
                                        resp.surrender_focus();
                                        *active_reg_edit = None;
                                    } else if interactive
                                        && (enter_pressed
                                            || resp.lost_focus()
                                            || resp.clicked_elsewhere())
                                    {
                                        resp.surrender_focus();
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
                                        RichText::new(&presentation.d[i].hex)
                                            .monospace()
                                            .color(col),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                draw_diff_pill(ui, label_resp.rect, changed);
                                if label_resp.clicked()
                                    && !ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    *active_reg_edit =
                                        Some((EditRegister::D(i), format!("{:08X}", val)));
                                }
                            }

                            let dec_str = &presentation.d[i].decimal;
                            ui.add(egui::Label::new(
                                RichText::new(dec_str).monospace().color(tokens.text_muted),
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
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for i in 0..cpu::state::REGISTER_COUNT {
                            let val = presentation.a[i].value;
                            let changed = presentation.a[i].changed;
                            let col = diff_color(changed);

                            let is_sup = presentation.supervisor;
                            let tooltip = if i == cpu::state::REGISTER_STACK_POINTER {
                                if is_sup {
                                    "Address register A7 (Active Supervisor Stack Pointer - SSP)"
                                } else {
                                    "Address register A7 (Active User Stack Pointer - USP)"
                                }
                            } else {
                                "Address register (32-bit)"
                            };

                            ui.monospace(A_LABELS[i]).on_hover_text(tooltip);

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
                                    let resp = ui
                                        .push_id("edit_a", |ui| {
                                            let r = ui.add(
                                                egui::TextEdit::singleline(buf)
                                                    .desired_width(75.0)
                                                    .font(egui::TextStyle::Monospace),
                                            );
                                            if interactive {
                                                r.request_focus();
                                            }
                                            r
                                        })
                                        .inner;

                                    if interactive && esc_pressed {
                                        resp.surrender_focus();
                                        *active_reg_edit = None;
                                    } else if interactive
                                        && (enter_pressed
                                            || resp.lost_focus()
                                            || resp.clicked_elsewhere())
                                    {
                                        resp.surrender_focus();
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
                                        RichText::new(&presentation.a[i].hex)
                                            .monospace()
                                            .color(col),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                draw_diff_pill(ui, label_resp.rect, changed);
                                if label_resp.clicked()
                                    && !ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    *active_reg_edit =
                                        Some((EditRegister::A(i), format!("{:08X}", val)));
                                }
                            }

                            let dec_str = &presentation.a[i].decimal;
                            ui.add(egui::Label::new(
                                RichText::new(dec_str).monospace().color(tokens.text_muted),
                            ));

                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });

                // Inactive stack pointer & active status
                let is_supervisor = presentation.supervisor;
                let (alt_label, alt_val, alt_id) = if is_supervisor {
                    ("Inactive USP:", presentation.alt.value, EditRegister::USP)
                } else {
                    ("Inactive SSP:", presentation.alt.value, EditRegister::SSP)
                };
                let alt_changed = presentation.alt.changed;

                ui.horizontal(|ui| {
                    ui.colored_label(
                        tokens.accent_pc,
                        if is_supervisor {
                            "A7 = SSP"
                        } else {
                            "A7 = USP"
                        },
                    )
                    .on_hover_text(if is_supervisor {
                        "A7 is currently bound to SSP (Supervisor Stack Pointer)"
                    } else {
                        "A7 is currently bound to USP (User Stack Pointer)"
                    });
                    ui.label(RichText::new("|").color(tokens.border_subtle));
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
                            let resp = ui
                                .push_id("edit_sp", |ui| {
                                    let r = ui.add(
                                        egui::TextEdit::singleline(buf)
                                            .desired_width(75.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    if interactive {
                                        r.request_focus();
                                    }
                                    r
                                })
                                .inner;

                            if interactive && esc_pressed {
                                resp.surrender_focus();
                                *active_reg_edit = None;
                            } else if interactive
                                && (enter_pressed || resp.lost_focus() || resp.clicked_elsewhere())
                            {
                                resp.surrender_focus();
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_val) = u32::from_str_radix(clean, 16) {
                                    if is_supervisor {
                                        state.set_usp(new_val);
                                    } else {
                                        state.set_ssp(new_val);
                                    }
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(&presentation.alt.hex)
                                    .monospace()
                                    .color(diff_color(alt_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, resp.rect, alt_changed);
                        if resp.clicked() && !ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
                let instruction_pc = presentation.pc.value;

                ui.columns(2, |cols| {
                    // Col 0: PC, SR, Mode, IPL
                    cols[0].vertical(|ui| {
                        ui.horizontal(|ui| {
                            let pc_changed = presentation.pc.changed;
                    ui.monospace("PC:")
                        .on_hover_ui(|ui| {
                            ui.heading("Program Counter (PC)");
                            ui.label(
                                &presentation.pc_tooltip,
                            );
                        });

                    let is_editing_pc = matches!(active_reg_edit, Some((EditRegister::PC, _)));
                    if is_editing_pc {
                        if let Some((_, buf)) = active_reg_edit {
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let resp = ui.push_id("edit_pc", |ui| {
                                let r = ui.add(
                                    egui::TextEdit::singleline(buf)
                                        .desired_width(70.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                if interactive { r.request_focus(); }
                                r
                            }).inner;

                            if interactive && esc_pressed {
                                resp.surrender_focus();
                                *active_reg_edit = None;
                            } else if interactive && (enter_pressed
                                || resp.lost_focus()
                                || resp.clicked_elsewhere())
                            {
                                resp.surrender_focus();
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_pc) = u32::from_str_radix(clean, 16) {
                                    pc_to_set = Some(new_pc & physical_memory::MASK_PHYSICAL_ADDRESS);
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let pc_resp = ui.add(
                            egui::Label::new(
                                RichText::new(&presentation.pc.hex)
                                    .monospace()
                                    .color(diff_color(pc_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, pc_resp.rect, pc_changed);
                        if pc_resp.clicked()
                            && !ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            *active_reg_edit =
                                Some((EditRegister::PC, format!("{:08X}", instruction_pc)));
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let sr_changed = presentation.sr.changed;
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
                            let resp = ui.push_id("edit_sr", |ui| {
                                let r = ui.add(
                                    egui::TextEdit::singleline(buf)
                                        .desired_width(45.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                if interactive { r.request_focus(); }
                                r
                            }).inner;

                            if interactive && esc_pressed {
                                resp.surrender_focus();
                                *active_reg_edit = None;
                            } else if interactive && (enter_pressed
                                || resp.lost_focus()
                                || resp.clicked_elsewhere())
                            {
                                resp.surrender_focus();
                                let clean = buf.trim().trim_start_matches('$');
                                if let Ok(new_sr) = u16::from_str_radix(clean, 16) {
                                    state.set_sr(new_sr);
                                }
                                *active_reg_edit = None;
                            }
                        }
                    } else {
                        let sr_resp = ui.add(
                            egui::Label::new(
                                RichText::new(&presentation.sr.hex)
                                    .monospace()
                                    .color(diff_color(sr_changed)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        draw_diff_pill(ui, sr_resp.rect, sr_changed);
                        if sr_resp.clicked()
                            && !ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            *active_reg_edit =
                                Some((EditRegister::SR, format!("{:04X}", state.sr())));
                        }
                    }
                });

                let is_supervisor = presentation.supervisor;
                let mode_str = if is_supervisor {
                    "Supervisor [S]"
                } else {
                    "User [U]"
                };
                let mode_color = if is_supervisor {
                    tokens.accent_warning
                } else {
                    tokens.accent_pc
                };
                ui.colored_label(mode_color, mode_str);

                ui.monospace(&presentation.ipl);
            });

            // Col 1: CCR Flags and Prefetch registers
            cols[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CCR:").strong())
                        .on_hover_text("Condition Code Register (CCR): Click any flag LED badge to toggle");

                    // Flags: X (bit 4), N (bit 3), Z (bit 2), V (bit 1), C (bit 0)
                    let flags = [
                        ("X", cpu::BIT_CCR_X, "eXtend: Used for multi-precision arithmetic. Preserved by data movement."),
                        ("N", cpu::BIT_CCR_N, "Negative: Set if result Most Significant Bit is 1 (negative)."),
                        ("Z", cpu::BIT_CCR_Z, "Zero: Set if result equals zero."),
                        ("V", cpu::BIT_CCR_V, "oVerflow: Set if signed two's complement overflow occurred."),
                        ("C", cpu::BIT_CCR_C, "Carry: Set if unsigned carry/borrow occurred during operation."),
                    ];

                    for (name, mask, desc) in flags {
                        let is_set = (presentation.sr.value as u16 & mask) != 0;
                        let flag_changed =
                            ((presentation.previous_sr ^ presentation.sr.value as u16) & mask) != 0;

                        let bg_color = if is_set {
                            tokens.ccr_active_bg
                        } else {
                            tokens.ccr_inactive_bg
                        };
                        let text_color = if is_set {
                            tokens.ccr_active_text
                        } else {
                            tokens.ccr_inactive_text
                        };

                        let mut btn =
                            egui::Button::new(RichText::new(name).color(text_color).strong())
                                .fill(bg_color)
                                .min_size(egui::vec2(18.0, 18.0));

                        if flag_changed {
                            btn = btn
                                .stroke(egui::Stroke::new(1.5_f32, tokens.accent_diff));
                        }

                        if ui.add(btn).on_hover_text(desc).clicked() {
                            state.set_sr(state.sr() ^ mask); // Toggle bit
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.monospace(&presentation.ir)
                        .on_hover_text("Instruction Register (IR): Current executing opcode");
                    ui.monospace(&presentation.irc)
                        .on_hover_text("Instruction Register Capture (IRC): Prefetched next pipeline word");
                });
            });
        });
            });
    });

    if let Some(target_pc) = pc_to_set {
        cpu.set_pc_and_prime_prefetch(target_pc, bus);
    }
    pc_to_set.is_some()
}
