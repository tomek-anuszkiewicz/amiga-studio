//! Right Dock: Breakpoints & Watchpoints Manager Panel
//!
//! Provides interactive UI to inspect, toggle, add, and delete PC execution breakpoints,
//! conditional register rules, and memory range watchpoints.

use debugger::{BreakpointCondition, ConditionOp, ConditionRegister, WatchAccess};
use egui::{Color32, RichText, Ui};

const REGISTER_DATA_FIRST: usize = 0;
const REGISTER_DATA_LAST: usize = cpu::state::REGISTER_COUNT - 1;
const REGISTER_ADDRESS_FIRST: usize = cpu::state::REGISTER_COUNT;
const REGISTER_ADDRESS_LAST: usize = 2 * cpu::state::REGISTER_COUNT - 1;
const REGISTER_PC_SELECTION: usize = 16;
const REGISTER_SR_SELECTION: usize = 17;
const EQ_SELECTION: usize = 0;
const NE_SELECTION: usize = 1;
const LT_SELECTION: usize = 2;
const GT_SELECTION: usize = 3;
const LTE_SELECTION: usize = 4;

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

#[derive(Debug, Default)]
pub(crate) struct BreakpointsPresentation {
    pc_count: String,
    watch_count: String,
    program_counters: Vec<(bool, String, Option<String>)>,
    watches: Vec<(bool, String, WatchAccess)>,
}

impl BreakpointsPresentation {
    fn refresh(&mut self, manager: &debugger::BreakpointManager) {
        self.pc_count = format!("({})", manager.pc_breakpoints.len());
        self.watch_count = format!("({})", manager.watchpoints.len());
        self.program_counters = manager
            .pc_breakpoints
            .iter()
            .map(|bp| {
                (
                    bp.enabled,
                    format!("${:06X}", bp.addr),
                    bp.condition
                        .as_ref()
                        .map(|c| format!("[IF {}]", c.format())),
                )
            })
            .collect();
        self.watches = manager
            .watchpoints
            .iter()
            .map(|wp| {
                (
                    wp.enabled,
                    format!("${:06X}..${:06X}", wp.start, wp.end),
                    wp.access,
                )
            })
            .collect();
    }
}

pub(crate) fn render_breakpoints_panel(
    bpm: &mut debugger::BreakpointManager,
    data: &mut BreakpointsPresentation,
    refresh: bool,
    form: &mut BreakpointFormState,
    ui: &mut Ui,
) {
    if refresh {
        data.refresh(bpm);
    }
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
                    ui.label(&data.pc_count);
                });
                ui.separator();

                if data.program_counters.is_empty() {
                    ui.label(
                        RichText::new(
                            "No PC breakpoints set. Click circle in Disassembly or add below.",
                        )
                        .italics()
                        .color(Color32::from_rgb(140, 145, 160)),
                    );
                } else {
                    let mut to_remove = None;
                    for (idx, (enabled, address, condition)) in
                        data.program_counters.iter_mut().enumerate()
                    {
                        ui.horizontal(|ui| {
                            if ui.checkbox(enabled, "").changed() {
                                bpm.pc_breakpoints[idx].enabled = *enabled;
                            }
                            let addr_color = if *enabled {
                                Color32::from_rgb(255, 80, 80)
                            } else {
                                Color32::from_rgb(120, 120, 130)
                            };
                            ui.monospace(RichText::new(&*address).color(addr_color));

                            if let Some(cond) = condition {
                                ui.colored_label(Color32::from_rgb(255, 180, 60), &*cond);
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
                                    REGISTER_DATA_FIRST..=REGISTER_DATA_LAST => {
                                        ConditionRegister::D(form.new_cond_reg_idx as u8)
                                    }
                                    REGISTER_ADDRESS_FIRST..=REGISTER_ADDRESS_LAST => {
                                        ConditionRegister::A(
                                            (form.new_cond_reg_idx - REGISTER_ADDRESS_FIRST) as u8,
                                        )
                                    }
                                    REGISTER_PC_SELECTION => ConditionRegister::PC,
                                    REGISTER_SR_SELECTION => ConditionRegister::SR,
                                    _ => ConditionRegister::CCR,
                                };
                                let op = match form.new_cond_op_idx {
                                    EQ_SELECTION => ConditionOp::Eq,
                                    NE_SELECTION => ConditionOp::Ne,
                                    LT_SELECTION => ConditionOp::Lt,
                                    GT_SELECTION => ConditionOp::Gt,
                                    LTE_SELECTION => ConditionOp::Lte,
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
                    ui.label(&data.watch_count);
                });
                ui.separator();

                if data.watches.is_empty() {
                    ui.label(
                        RichText::new(
                            "No watchpoints active. Add address range below to watch reads/writes.",
                        )
                        .italics()
                        .color(Color32::from_rgb(140, 145, 160)),
                    );
                } else {
                    let mut wp_to_remove = None;
                    for (idx, (enabled, address, access)) in data.watches.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.checkbox(enabled, "").changed() {
                                bpm.watchpoints[idx].enabled = *enabled;
                            }
                            ui.monospace(&*address);
                            let (access_label, access_color) = match access {
                                WatchAccess::Read => ("[READ]", Color32::from_rgb(80, 180, 255)),
                                WatchAccess::Write => ("[WRITE]", Color32::from_rgb(255, 100, 100)),
                                WatchAccess::Any => ("[ANY]", Color32::from_rgb(255, 200, 50)),
                            };
                            ui.colored_label(access_color, access_label);

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
