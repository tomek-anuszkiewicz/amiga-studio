//! Right Dock: Breakpoints & Watchpoints Manager Panel
//!
//! Provides interactive UI to inspect, toggle, add, and delete PC execution breakpoints,
//! conditional register rules, and memory range watchpoints.

use debugger::{BreakpointCondition, ConditionOp, ConditionRegister, WatchAccess};
use egui::{Color32, RichText, Ui};

/// State for the inline Breakpoint/Watchpoint creation form
#[derive(Debug, Clone)]
pub struct BreakpointFormState {
    pub new_pc_str: String,
    pub new_cond_enabled: bool,
    pub new_cond_reg_idx: usize, // 0..8 (D0-D7), 8..16 (A0-A7), 16: PC, 17: SR, 18: CCR
    pub new_cond_op_idx: usize,  // 0: ==, 1: !=, 2: <, 3: >, 4: <=, 5: >=
    pub new_cond_val_str: String,

    pub new_wp_start_str: String,
    pub new_wp_end_str: String,
    pub new_wp_access: WatchAccess,
}

impl Default for BreakpointFormState {
    fn default() -> Self {
        Self {
            new_pc_str: "001000".to_string(),
            new_cond_enabled: false,
            new_cond_reg_idx: 0,
            new_cond_op_idx: 0,
            new_cond_val_str: "0".to_string(),
            new_wp_start_str: "002000".to_string(),
            new_wp_end_str: "0020FF".to_string(),
            new_wp_access: WatchAccess::Write,
        }
    }
}

pub fn render_breakpoints_panel(
    bpm: &mut debugger::BreakpointManager,
    form: &mut BreakpointFormState,
    ui: &mut Ui,
) {
    ui.spacing_mut().indent = 0.0;
    egui::CollapsingHeader::new(RichText::new("🎯 Breakpoints & Watchpoints").strong())
        .default_open(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_space(4.0);

            // --- Section 1: PC Execution Breakpoints ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PC Breakpoints")
                            .strong()
                            .color(Color32::from_rgb(0, 229, 255)),
                    );
                    ui.label(format!("({})", bpm.pc_breakpoints.len()));
                });
                ui.separator();

                if bpm.pc_breakpoints.is_empty() {
                    ui.label(
                        RichText::new(
                            "No PC breakpoints set. Click circle in Disassembly or add below.",
                        )
                        .italics()
                        .color(Color32::from_rgb(140, 145, 160)),
                    );
                } else {
                    let mut to_remove = None;
                    for (idx, bp) in bpm.pc_breakpoints.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut bp.enabled, "");
                            let addr_color = if bp.enabled {
                                Color32::from_rgb(255, 80, 80)
                            } else {
                                Color32::from_rgb(120, 120, 130)
                            };
                            ui.monospace(
                                RichText::new(format!("${:06X}", bp.addr)).color(addr_color),
                            );

                            if let Some(ref cond) = bp.condition {
                                ui.colored_label(
                                    Color32::from_rgb(255, 180, 60),
                                    format!("[IF {}]", cond.format()),
                                );
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(
                                            RichText::new("✕")
                                                .color(Color32::from_rgb(220, 80, 80)),
                                        )
                                        .clicked()
                                    {
                                        to_remove = Some(idx);
                                    }
                                },
                            );
                        });
                    }
                    if let Some(idx) = to_remove {
                        bpm.pc_breakpoints.remove(idx);
                    }
                }

                ui.add_space(6.0);
                // Inline PC Breakpoint Creator
                ui.collapsing("➕ Add New PC Breakpoint", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("PC ($):");
                        ui.add(
                            egui::TextEdit::singleline(&mut form.new_pc_str)
                                .desired_width(70.0)
                                .font(egui::TextStyle::Monospace),
                        );
                    });

                    ui.checkbox(&mut form.new_cond_enabled, "Attach Condition (Register)");
                    if form.new_cond_enabled {
                        ui.horizontal(|ui| {
                            let reg_labels = [
                                "D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7", "A0", "A1", "A2",
                                "A3", "A4", "A5", "A6", "A7", "PC", "SR", "CCR",
                            ];
                            egui::ComboBox::from_id_salt("cond_reg")
                                .selected_text(
                                    reg_labels[form.new_cond_reg_idx.min(reg_labels.len() - 1)],
                                )
                                .show_index(
                                    ui,
                                    &mut form.new_cond_reg_idx,
                                    reg_labels.len(),
                                    |i| reg_labels[i],
                                );

                            let op_labels = ["==", "!=", "<", ">", "<=", ">="];
                            egui::ComboBox::from_id_salt("cond_op")
                                .selected_text(
                                    op_labels[form.new_cond_op_idx.min(op_labels.len() - 1)],
                                )
                                .show_index(ui, &mut form.new_cond_op_idx, op_labels.len(), |i| {
                                    op_labels[i]
                                });

                            ui.label("$");
                            ui.add(
                                egui::TextEdit::singleline(&mut form.new_cond_val_str)
                                    .desired_width(60.0)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                    }

                    if ui.button("Add PC Breakpoint").clicked() {
                        if let Ok(addr) =
                            u32::from_str_radix(form.new_pc_str.trim().trim_start_matches('$'), 16)
                        {
                            if form.new_cond_enabled {
                                let val = u32::from_str_radix(
                                    form.new_cond_val_str.trim().trim_start_matches('$'),
                                    16,
                                )
                                .unwrap_or(0);
                                let reg = match form.new_cond_reg_idx {
                                    0..=7 => ConditionRegister::D(form.new_cond_reg_idx as u8),
                                    8..=15 => {
                                        ConditionRegister::A((form.new_cond_reg_idx - 8) as u8)
                                    }
                                    16 => ConditionRegister::PC,
                                    17 => ConditionRegister::SR,
                                    _ => ConditionRegister::CCR,
                                };
                                let op = match form.new_cond_op_idx {
                                    0 => ConditionOp::Eq,
                                    1 => ConditionOp::Ne,
                                    2 => ConditionOp::Lt,
                                    3 => ConditionOp::Gt,
                                    4 => ConditionOp::Lte,
                                    _ => ConditionOp::Gte,
                                };
                                bpm.add_conditional_breakpoint(
                                    addr,
                                    BreakpointCondition {
                                        register: reg,
                                        op,
                                        value: val,
                                        mask: None,
                                    },
                                );
                            } else {
                                bpm.add_pc_breakpoint(addr);
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // --- Section 2: Memory Watchpoints ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Memory Watchpoints")
                            .strong()
                            .color(Color32::from_rgb(255, 180, 0)),
                    );
                    ui.label(format!("({})", bpm.watchpoints.len()));
                });
                ui.separator();

                if bpm.watchpoints.is_empty() {
                    ui.label(
                        RichText::new(
                            "No watchpoints active. Add address range below to watch reads/writes.",
                        )
                        .italics()
                        .color(Color32::from_rgb(140, 145, 160)),
                    );
                } else {
                    let mut wp_to_remove = None;
                    for (idx, wp) in bpm.watchpoints.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut wp.enabled, "");
                            ui.monospace(format!("${:06X}..${:06X}", wp.start, wp.end));
                            let (access_label, access_color) = match wp.access {
                                WatchAccess::Read => ("READ", Color32::from_rgb(80, 180, 255)),
                                WatchAccess::Write => ("WRITE", Color32::from_rgb(255, 100, 100)),
                                WatchAccess::Any => ("ANY", Color32::from_rgb(255, 200, 50)),
                            };
                            ui.colored_label(access_color, format!("[{}]", access_label));

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(
                                            RichText::new("✕")
                                                .color(Color32::from_rgb(220, 80, 80)),
                                        )
                                        .clicked()
                                    {
                                        wp_to_remove = Some(idx);
                                    }
                                },
                            );
                        });
                    }
                    if let Some(idx) = wp_to_remove {
                        bpm.remove_watchpoint(idx);
                    }
                }

                ui.add_space(6.0);
                // Inline Watchpoint Creator
                ui.collapsing("➕ Add New Watchpoint", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Start ($):");
                        ui.add(
                            egui::TextEdit::singleline(&mut form.new_wp_start_str)
                                .desired_width(70.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.label("End ($):");
                        ui.add(
                            egui::TextEdit::singleline(&mut form.new_wp_end_str)
                                .desired_width(70.0)
                                .font(egui::TextStyle::Monospace),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Access:");
                        ui.radio_value(&mut form.new_wp_access, WatchAccess::Write, "Write");
                        ui.radio_value(&mut form.new_wp_access, WatchAccess::Read, "Read");
                        ui.radio_value(&mut form.new_wp_access, WatchAccess::Any, "Any");
                    });

                    if ui.button("Add Watchpoint").clicked() {
                        let start_ok = u32::from_str_radix(
                            form.new_wp_start_str.trim().trim_start_matches('$'),
                            16,
                        );
                        let end_ok = u32::from_str_radix(
                            form.new_wp_end_str.trim().trim_start_matches('$'),
                            16,
                        );
                        if let (Ok(start), Ok(end)) = (start_ok, end_ok) {
                            bpm.add_watchpoint(start, end, form.new_wp_access);
                        }
                    }
                });
            });
        });
}
