//! Left Dock: Disassembly View Component
//!
//! M68000 instruction disassembly stream, execution cursor tracking, breakpoints,
//! and in-place instruction editing with byte size invariance enforcement.

use crate::theme::ColorTokens;
use debugger::{assemble_instruction, disassemble, find_aligned_disassembly_start, Debugger};
use egui::{RichText, Ui};
use m68000::Cpu;
use memory_bus::MemoryBus;

/// State for inline instruction editing in disassembly table
#[derive(Debug, Clone)]
pub struct DisasmEditState {
    pub addr: u32,
    pub text: String,
    pub error: Option<String>,
}

pub fn render_disassembly(
    ui: &mut Ui,
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    debugger: &mut Debugger,
    temporal: &mut debugger::temporal::TemporalHistory,
    goto_addr_str: &mut String,
    active_edit: &mut Option<DisasmEditState>,
    tokens: &ColorTokens,
) {
    ui.heading("Disassembly");

    let current_pc = cpu.state.instruction_pc & 0x00FF_FFFF;

    // 1. Navigation & Jump Bar
    ui.horizontal(|ui| {
        ui.label("Goto:");
        let response = ui.add(
            egui::TextEdit::singleline(goto_addr_str)
                .desired_width(70.0)
                .hint_text("001000"),
        );
        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || ui.button("Jump").clicked()
        {
            let clean = goto_addr_str.trim().trim_start_matches('$');
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                cpu.set_pc_and_prime_prefetch(addr & 0x00FF_FFFF, bus);
            }
        }

        if ui.button("Current PC").clicked() {
            *goto_addr_str = format!("{:06X}", current_pc);
        }
    });

    ui.separator();

    // 2. Disassembly Table (Smart aligned stream with CISC boundary preservation)
    let mut known_boundaries: Vec<u32> = Vec::with_capacity(32);
    let temp_len = temporal.len();
    for i in 0..temp_len.min(32) {
        if let Some(frame) = temporal.get_chronological(temp_len - 1 - i) {
            known_boundaries.push(frame.pc);
        }
    }
    let trace_len = debugger.trace.len();
    for i in 0..trace_len.min(32) {
        if let Some(entry) = debugger.trace.get(trace_len - 1 - i) {
            if !known_boundaries.contains(&entry.pc) {
                known_boundaries.push(entry.pc);
            }
        }
    }

    let start_addr = find_aligned_disassembly_start(
        current_pc,
        3,
        |a| bus.read_word_debug(a),
        &known_boundaries,
    );
    let mut cur_addr = start_addr;

    egui::ScrollArea::vertical()
        .id_salt("disassembly_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let row_count = ((ui.available_height() / 19.0).max(35.0) as usize).min(100);
            for _ in 0..row_count {
                let (disasm, byte_len) = disassemble(cur_addr, |a| bus.read_word_debug(a));
                let is_current = cur_addr == current_pc;
                let is_bp = debugger.breakpoints.check_pc(cur_addr);
                let orig_len = disasm.word_count * 2;

                let row_frame = if is_current {
                    egui::Frame::NONE
                        .fill(tokens.accent_pc_bg)
                        .stroke(egui::Stroke::new(1.0_f32, tokens.accent_pc))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(3, 1))
                } else {
                    egui::Frame::NONE.inner_margin(egui::Margin::symmetric(3, 1))
                };

                let is_editing_this = active_edit.as_ref().map_or(false, |e| e.addr == cur_addr);

                row_frame.show(ui, |ui| {
                    if is_editing_this {
                        // Inline instruction edit mode
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.monospace(
                                    RichText::new(format!("{:06X}:", cur_addr))
                                        .color(tokens.accent_pc),
                                );

                                let mut commit = false;
                                let mut cancel = false;

                                if let Some(edit) = active_edit.as_mut() {
                                    let edit_resp = ui.add(
                                        egui::TextEdit::singleline(&mut edit.text)
                                            .desired_width(180.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    edit_resp.request_focus();
                                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        commit = true;
                                    }
                                    if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                        || edit_resp.clicked_elsewhere()
                                    {
                                        cancel = true;
                                    }

                                    if ui.button("✓ Save").clicked() {
                                        commit = true;
                                    }
                                    if ui.button("✕ Cancel").clicked() {
                                        cancel = true;
                                    }
                                }

                                if cancel {
                                    *active_edit = None;
                                } else if commit {
                                    if let Some(edit) = active_edit.as_mut() {
                                        match assemble_instruction(&edit.text, cur_addr) {
                                            Err(err) => {
                                                edit.error = Some(err);
                                            }
                                            Ok(words) => {
                                                let new_len = words.len() * 2;
                                                if new_len != orig_len {
                                                    edit.error = Some(format!(
                                                        "Byte size mismatch: original is {} bytes ({} words), but new is {} bytes ({} words). In-place edit requires exact byte count match.",
                                                        orig_len,
                                                        orig_len / 2,
                                                        new_len,
                                                        new_len / 2
                                                    ));
                                                } else {
                                                    // Commit words to physical memory
                                                    for (i, word) in words.iter().enumerate() {
                                                        let w_addr = cur_addr.wrapping_add((i * 2) as u32);
                                                        bus.write_word_debug(w_addr, *word);
                                                    }

                                                    // If this is the current instruction, re-prime prefetch
                                                    if is_current {
                                                        cpu.set_pc_and_prime_prefetch(cur_addr, bus);
                                                    }
                                                    *active_edit = None;
                                                }
                                            }
                                        }
                                    }
                                }
                            });

                            // Display validation error if present
                            if let Some(edit) = active_edit.as_ref() {
                                if let Some(err) = &edit.error {
                                    ui.colored_label(tokens.accent_error, format!("❌ {}", err));
                                }
                            }
                        });
                    } else {
                        // Standard display mode
                        ui.horizontal(|ui| {
                            // Breakpoint toggle dot
                            let bp_color = if is_bp {
                                tokens.accent_error
                            } else if is_current {
                                tokens.accent_pc
                            } else {
                                tokens.text_muted
                            };
                            let bp_btn = ui.selectable_label(is_bp, RichText::new("●").color(bp_color));
                            if bp_btn.clicked() {
                                if is_bp {
                                    debugger.breakpoints.remove_pc_breakpoint(cur_addr);
                                } else {
                                    debugger.breakpoints.add_pc_breakpoint(cur_addr);
                                }
                            }

                            // Address & Disassembly text (guaranteed uniform column offset on every row)
                            let line_text = if is_current {
                                RichText::new(disasm.format_line()).monospace().strong().color(tokens.text_primary)
                            } else {
                                RichText::new(disasm.format_line()).monospace().color(tokens.text_secondary)
                            };
                            let line_label = ui
                                .add(egui::Label::new(line_text).sense(egui::Sense::click()))
                                .on_hover_text(
                                    "Double-click to edit instruction in-place | Right-click for options",
                                );

                            if line_label.double_clicked() {
                                let initial_text = if disasm.operands.is_empty() {
                                    disasm.mnemonic.to_string()
                                } else {
                                    format!("{} {}", disasm.mnemonic, disasm.operands)
                                };
                                *active_edit = Some(DisasmEditState {
                                    addr: cur_addr,
                                    text: initial_text,
                                    error: None,
                                });
                            }

                            // Quick Loop Rewind button (rendered AFTER disassembly text so columns stay aligned)
                            let historical_passes = temporal.find_matches_by_pc(cur_addr, 10);
                            if !historical_passes.is_empty() {
                                if ui
                                    .button(
                                        RichText::new("⏪")
                                            .size(11.0)
                                            .color(tokens.accent_diff),
                                    )
                                    .on_hover_text(format!(
                                        "Rewind to historical execution ({} passes recorded)",
                                        historical_passes.len()
                                    ))
                                    .clicked()
                                {
                                    if let Some((last_idx, _)) = historical_passes.last() {
                                        if let Some(target_frame) = temporal.scrub_to_index(*last_idx) {
                                            cpu.state = target_frame.state.clone();
                                        }
                                    }
                                }
                            }

                            // Right-click Context Menu
                            line_label.context_menu(|ui| {
                                if ui.button("📍 Set PC here").clicked() {
                                    cpu.set_pc_and_prime_prefetch(cur_addr, bus);
                                    ui.close_menu();
                                }
                                let bp_label = if is_bp { "● Remove Breakpoint" } else { "● Set Breakpoint" };
                                if ui.button(bp_label).clicked() {
                                    if is_bp {
                                        debugger.breakpoints.remove_pc_breakpoint(cur_addr);
                                    } else {
                                        debugger.breakpoints.add_pc_breakpoint(cur_addr);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("✏ Edit Instruction").clicked() {
                                    let initial_text = if disasm.operands.is_empty() {
                                        disasm.mnemonic.to_string()
                                    } else {
                                        format!("{} {}", disasm.mnemonic, disasm.operands)
                                    };
                                    *active_edit = Some(DisasmEditState {
                                        addr: cur_addr,
                                        text: initial_text,
                                        error: None,
                                    });
                                    ui.close_menu();
                                }

                                if !historical_passes.is_empty() {
                                    ui.separator();
                                    ui.label(
                                        RichText::new("⏪ Rewind to Historical Pass:")
                                            .small()
                                            .color(tokens.accent_diff),
                                    );
                                    for (pass_num, (hist_idx, cck)) in historical_passes.iter().enumerate() {
                                        let item_label = format!("Pass #{} (CCK: {})", pass_num + 1, cck);
                                        if ui.button(item_label).clicked() {
                                            if let Some(target_frame) = temporal.scrub_to_index(*hist_idx) {
                                                cpu.state = target_frame.state.clone();
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });

                            // Double-click to jump PC to this address
                            if line_label.double_clicked() {
                                cpu.set_pc_and_prime_prefetch(cur_addr, bus);
                            }
                        });
                    }
                });

                cur_addr = cur_addr.wrapping_add(byte_len.max(2));
            }
        });
}
