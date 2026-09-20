//! Left Dock: Disassembly View Component
//!
//! M68000 instruction disassembly stream, execution cursor tracking, breakpoints,
//! and in-place instruction editing with byte size invariance enforcement.

use crate::theme::ColorTokens;
use cpu::Cpu;
use debugger::{assemble_instruction, disassemble, find_aligned_disassembly_start, Debugger};
use egui::{RichText, Ui};
use physical_memory::PhysicalMemory;

/// State for inline instruction editing in disassembly table
#[derive(Debug, Clone)]
pub struct DisasmEditState {
    pub addr: u32,
    pub text: String,
    pub error: Option<String>,
}

pub(crate) fn render_disassembly(
    ui: &mut Ui,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
    debugger: &mut Debugger,
    temporal: &mut debugger::temporal::TemporalHistory,
    goto_addr_str: &mut String,
    view_addr: &mut Option<u32>,
    selected_addr: &mut Option<u32>,
    active_edit: &mut Option<DisasmEditState>,
    tokens: &ColorTokens,
) {
    let start_pos = ui.cursor().min;
    let disasm_rect_id = egui::Id::new("disasm_view_rect");
    let prev_rect: Option<egui::Rect> = ui.data(|d| d.get_temp(disasm_rect_id));
    let is_hovered = prev_rect.map_or(false, |r| ui.rect_contains_pointer(r));

    ui.heading("Disassembly");

    let current_pc = cpu.state.instruction_pc & 0x00FF_FFFF;

    // 1. Navigation & Jump Bar
    ui.horizontal(|ui| {
        ui.label("Goto:");
        let response = ui.add(
            egui::TextEdit::singleline(goto_addr_str)
                .desired_width(70.0)
                .hint_text("000000"),
        );
        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || ui.button("Jump").clicked()
        {
            let clean = goto_addr_str.trim().trim_start_matches('$');
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                let target = addr & 0x00FF_FFFF;
                cpu.set_pc_and_prime_prefetch(target, bus);
                *view_addr = None;
                *selected_addr = Some(target);
            }
        }

        if ui.button("Current PC").clicked() {
            *goto_addr_str = format!("{:06X}", current_pc);
            *view_addr = None;
            *selected_addr = Some(current_pc);
        }

        if view_addr.is_some() && ui.button("Track PC").clicked() {
            *view_addr = None;
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

    let default_start = find_aligned_disassembly_start(
        current_pc,
        3,
        |a| bus.read_word_debug(a),
        &known_boundaries,
    );

    // Mouse wheel infinite stream browsing
    if is_hovered {
        let mut scrolled = false;
        for event in ui.input(|i| i.events.clone()) {
            if let egui::Event::MouseWheel {
                delta, modifiers, ..
            } = event
            {
                let clicks = (-delta.y.round()) as i32;
                let multiplier = if modifiers.ctrl || modifiers.command {
                    10
                } else if modifiers.shift {
                    5
                } else {
                    1
                };
                let steps = (clicks.unsigned_abs() as usize) * multiplier;
                if clicks > 0 {
                    let mut curr = view_addr.unwrap_or(default_start);
                    for _ in 0..steps.min(50) {
                        let (_, len) = disassemble(curr, |a| bus.read_word_debug(a));
                        curr = curr.wrapping_add(len.max(2)) & 0x00FF_FFFF;
                    }
                    *view_addr = Some(curr);
                    scrolled = true;
                } else if clicks < 0 {
                    let mut curr = view_addr.unwrap_or(default_start);
                    for _ in 0..steps.min(50) {
                        let prev = find_aligned_disassembly_start(
                            curr,
                            1,
                            |a| bus.read_word_debug(a),
                            &known_boundaries,
                        );
                        if prev >= curr {
                            curr = curr.wrapping_sub(2) & 0x00FF_FFFE;
                        } else {
                            curr = prev;
                        }
                    }
                    *view_addr = Some(curr);
                    scrolled = true;
                }
            }
        }

        if scrolled {
            ui.input_mut(|i| {
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                i.raw_scroll_delta = egui::Vec2::ZERO;
            });
        }

        // Keyboard navigation across instruction rows
        if active_edit.is_none() {
            if let Some(sel) = *selected_addr {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    let (_, len) = disassemble(sel, |a| bus.read_word_debug(a));
                    *selected_addr = Some(sel.wrapping_add(len.max(2)) & 0x00FF_FFFF);
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    let prev = find_aligned_disassembly_start(
                        sel,
                        1,
                        |a| bus.read_word_debug(a),
                        &known_boundaries,
                    );
                    let next_sel = if prev >= sel {
                        sel.wrapping_sub(2) & 0x00FF_FFFE
                    } else {
                        prev
                    };
                    *selected_addr = Some(next_sel);
                } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let (disasm, _) = disassemble(sel, |a| bus.read_word_debug(a));
                    let initial_text = if disasm.operands.is_empty() {
                        disasm.mnemonic.to_string()
                    } else {
                        format!("{} {}", disasm.mnemonic, disasm.operands)
                    };
                    *active_edit = Some(DisasmEditState {
                        addr: sel,
                        text: initial_text,
                        error: None,
                    });
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *selected_addr = None;
                }
            }
        }
    }

    let anchor_addr = view_addr.unwrap_or(default_start) & 0x00FF_FFFF;
    let mut cur_addr = anchor_addr;

    let total_avail_h = ui.available_height();
    const BOTTOM_MARGIN: f32 = 10.0;
    const ROW_HEIGHT: f32 = 21.5;
    let usable_h = (total_avail_h - BOTTOM_MARGIN).max(ROW_HEIGHT * 4.0);
    let row_count = ((usable_h / ROW_HEIGHT).floor() as usize).max(4);

    const SCROLLBAR_WIDTH: f32 = 8.0;
    const SPACING_X: f32 = 2.0;
    let list_width = (ui.available_width() - SCROLLBAR_WIDTH - SPACING_X).max(80.0);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.x = SPACING_X;
        ui.allocate_ui_with_layout(
            egui::vec2(list_width, usable_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.set_min_width(list_width);
                ui.set_width(list_width);
                for _ in 0..row_count {
                let (disasm, byte_len) = disassemble(cur_addr, |a| bus.read_word_debug(a));
                let is_current = cur_addr == current_pc;
                let is_selected = *selected_addr == Some(cur_addr);
                let is_bp = debugger.breakpoints.check_pc(cur_addr);
                let orig_len = disasm.word_count * 2;

                let row_frame = if is_current && is_selected {
                    egui::Frame::NONE
                        .fill(tokens.accent_pc_bg)
                        .stroke(egui::Stroke::new(1.5_f32, tokens.border_active))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(3, 1))
                } else if is_current {
                    egui::Frame::NONE
                        .fill(tokens.accent_pc_bg)
                        .stroke(egui::Stroke::new(1.0_f32, tokens.accent_pc))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(3, 1))
                } else if is_selected {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgba_unmultiplied(
                            tokens.border_active.r(),
                            tokens.border_active.g(),
                            tokens.border_active.b(),
                            35,
                        ))
                        .stroke(egui::Stroke::new(1.0_f32, tokens.border_active))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(3, 1))
                } else {
                    egui::Frame::NONE.inner_margin(egui::Margin::symmetric(3, 1))
                };

                let is_editing_this = active_edit.as_ref().map_or(false, |e| e.addr == cur_addr);

                row_frame.show(ui, |ui| {
                    if is_editing_this {
                        // In-place inline instruction edit mode
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

                                    if ui.button(RichText::new("Save").strong()).clicked() {
                                        commit = true;
                                    }
                                    if ui.button("Cancel").clicked() {
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
                                    ui.colored_label(tokens.accent_error, format!("Error: {}", err));
                                }
                            }
                        });
                    } else {
                        // Standard display mode
                        ui.horizontal(|ui| {
                            // Vector painted breakpoint toggle indicator (crisp circle, zero font glyph fallback)
                            let (bp_rect, bp_resp) = ui.allocate_exact_size(
                                egui::vec2(12.0, 16.0),
                                egui::Sense::click(),
                            );
                            let bp_center = bp_rect.center();
                            if is_bp {
                                ui.painter().circle_filled(bp_center, 4.5, tokens.accent_error);
                            } else if bp_resp.hovered() {
                                ui.painter().circle_stroke(
                                    bp_center,
                                    4.0,
                                    egui::Stroke::new(1.2_f32, tokens.accent_pc),
                                );
                            } else if is_current {
                                ui.painter().circle_filled(
                                    bp_center,
                                    2.5,
                                    tokens.accent_pc,
                                );
                            } else {
                                ui.painter().circle_stroke(
                                    bp_center,
                                    3.0,
                                    egui::Stroke::new(1.0_f32, tokens.text_muted),
                                );
                            }
                            if bp_resp.clicked() {
                                if is_bp {
                                    debugger.breakpoints.remove_pc_breakpoint(cur_addr);
                                } else {
                                    debugger.breakpoints.add_pc_breakpoint(cur_addr);
                                }
                            }

                            // Address & Disassembly text
                            let line_text = if is_current {
                                RichText::new(disasm.format_line()).monospace().strong().color(tokens.text_primary)
                            } else {
                                RichText::new(disasm.format_line()).monospace().color(tokens.text_secondary)
                            };
                            let line_label = ui
                                .add(egui::Label::new(line_text).sense(egui::Sense::click()))
                                .on_hover_text(
                                    "Single-click to select | Double-click to edit | Right-click for options",
                                );

                            if line_label.clicked() {
                                *selected_addr = Some(cur_addr);
                            }

                            if line_label.double_clicked() {
                                *selected_addr = Some(cur_addr);
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

                            // Quick Loop Rewind button
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
                                            cpu.restore_state(target_frame.state.clone());
                                        }
                                    }
                                }
                            }

                            // Right-click Context Menu
                            line_label.context_menu(|ui| {
                                if ui.button("📍 Set PC here").clicked() {
                                    cpu.set_pc_and_prime_prefetch(cur_addr, bus);
                                    *view_addr = None;
                                    *selected_addr = Some(cur_addr);
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
                                                cpu.restore_state(target_frame.state.clone());
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });
                        });
                    }
                });

                    cur_addr = (cur_addr.wrapping_add(byte_len.max(2))) & 0x00FF_FFFF;
                }
            },
        );

        // 24-bit vertical scrollbar on the right edge
        render_disasm_scrollbar(ui, view_addr, default_start, usable_h, tokens);
    });

    ui.add_space(BOTTOM_MARGIN);

    let end_pos = ui.cursor().min;
    let total_rect = egui::Rect::from_min_max(
        start_pos,
        egui::pos2(ui.max_rect().max.x, end_pos.y.max(start_pos.y + 100.0)),
    );
    ui.data_mut(|d| d.insert_temp(disasm_rect_id, total_rect));
}

/// Interactive vertical scrollbar mapping 24-bit Amiga address space ($000000..=$00FFFFFE)
fn render_disasm_scrollbar(
    ui: &mut Ui,
    view_addr: &mut Option<u32>,
    current_addr: u32,
    track_height: f32,
    tokens: &ColorTokens,
) {
    const MAX_ADDR: f32 = 0x00FF_FFFE as f32;
    const SCROLLBAR_WIDTH: f32 = 8.0;

    let (track_rect, track_response) = ui.allocate_exact_size(
        egui::vec2(SCROLLBAR_WIDTH, track_height),
        egui::Sense::click_and_drag(),
    );

    let active_addr = view_addr.unwrap_or(current_addr);
    let thumb_height = 24.0_f32.max(track_height * 0.08);
    let fraction = (active_addr as f32 / MAX_ADDR).clamp(0.0, 1.0);
    let travel = (track_rect.height() - thumb_height).max(1.0);
    let thumb_top = track_rect.top() + fraction * travel;
    let thumb_rect = egui::Rect::from_min_size(
        egui::pos2(track_rect.left() + 1.0, thumb_top),
        egui::vec2(track_rect.width() - 2.0, thumb_height),
    );

    // Track background
    ui.painter().rect_filled(
        track_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(20, 24, 34, 180),
    );
    ui.painter().rect_stroke(
        track_rect,
        2.0,
        egui::Stroke::new(1.0_f32, tokens.border_subtle),
        egui::StrokeKind::Inside,
    );

    // Handle drag
    if track_response.dragged() {
        if let Some(ptr) = ui.input(|i| i.pointer.latest_pos()) {
            let rel_y = (ptr.y - track_rect.top() - thumb_height / 2.0).clamp(0.0, travel);
            let frac = rel_y / travel;
            *view_addr = Some(((frac * MAX_ADDR) as u32) & 0x00FF_FFFE);
        }
    } else if track_response.clicked() {
        if let Some(click_pos) = track_response.interact_pointer_pos() {
            if click_pos.y < thumb_rect.top() {
                let curr = view_addr.unwrap_or(current_addr);
                *view_addr = Some(curr.wrapping_sub(256) & 0x00FF_FFFE);
            } else if click_pos.y > thumb_rect.bottom() {
                let curr = view_addr.unwrap_or(current_addr);
                *view_addr = Some(curr.wrapping_add(256) & 0x00FF_FFFE);
            }
        }
    }

    let thumb_color = if track_response.dragged() {
        tokens.border_active
    } else if track_response.hovered() {
        tokens.text_secondary
    } else {
        tokens.border_subtle
    };

    ui.painter().rect_filled(thumb_rect, 2.0, thumb_color);

    track_response.on_hover_text(format!(
        "Disassembly: ${:06X}\nDrag to scroll across 24-bit memory space",
        active_addr
    ));
}
