//! Right Dock: Memory Hex Editor Component
//!
//! 16-byte hex + ASCII viewer and editor with fast chunk navigation buttons,
//! infinite mouse wheel scrolling, keyboard navigation, vertical interactive scrollbar,
//! inline interactive editing, and step diff highlighting.

use crate::theme::ColorTokens;
use debugger::{BreakpointManager, WatchAccess};
use egui::{Color32, RichText, Ui};
use memory_bus::MemoryBus;

pub fn render_memory_hex(
    ui: &mut Ui,
    bus: &mut MemoryBus,
    breakpoints: &mut BreakpointManager,
    base_addr: &mut u32,
    edit_buffer: &mut (u32, String),
    prev_memory: Option<(&[u8; 256], u32)>,
    tokens: &ColorTokens,
) {
    let start_pos = ui.cursor().min;
    let mem_hex_rect_id = egui::Id::new("mem_hex_rect");
    let prev_rect: Option<egui::Rect> = ui.data(|d| d.get_temp(mem_hex_rect_id));
    let is_hovered = prev_rect.map_or(false, |r| ui.rect_contains_pointer(r));

    // 0. Mouse Wheel Infinite Scroll & Keyboard Navigation Handling
    if is_hovered {
        let mut scrolled = false;

        for event in ui.input(|i| i.events.clone()) {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } = event
            {
                let multiplier: u32 = if modifiers.ctrl && modifiers.shift {
                    256 // 4 KB per notch
                } else if modifiers.ctrl || modifiers.command {
                    64 // 1 KB per notch
                } else if modifiers.shift {
                    16 // 256 bytes (1 full page) per notch
                } else if modifiers.alt {
                    4 // 64 bytes per notch
                } else {
                    1 // 1 row (16 bytes) per notch
                };

                match unit {
                    egui::MouseWheelUnit::Line => {
                        let clicks = (-delta.y.round()) as i32;
                        if clicks > 0 {
                            *base_addr = base_addr.wrapping_add((clicks as u32) * multiplier * 16)
                                & 0x00FF_FFF0;
                            scrolled = true;
                        } else if clicks < 0 {
                            *base_addr = base_addr
                                .wrapping_sub(((-clicks) as u32) * multiplier * 16)
                                & 0x00FF_FFF0;
                            scrolled = true;
                        }
                    }
                    egui::MouseWheelUnit::Point | egui::MouseWheelUnit::Page => {
                        let accum_id = ui.id().with("hex_scroll_accum");
                        let mut accum: f32 = ui.data(|d| d.get_temp(accum_id)).unwrap_or(0.0);
                        accum += delta.y;
                        const POINTS_PER_ROW: f32 = 18.0;
                        if accum <= -POINTS_PER_ROW {
                            let rows = (-accum / POINTS_PER_ROW) as u32;
                            accum += (rows as f32) * POINTS_PER_ROW;
                            *base_addr =
                                base_addr.wrapping_add(rows * multiplier * 16) & 0x00FF_FFF0;
                            scrolled = true;
                        } else if accum >= POINTS_PER_ROW {
                            let rows = (accum / POINTS_PER_ROW) as u32;
                            accum -= (rows as f32) * POINTS_PER_ROW;
                            *base_addr =
                                base_addr.wrapping_sub(rows * multiplier * 16) & 0x00FF_FFF0;
                            scrolled = true;
                        }
                        ui.data_mut(|d| d.insert_temp(accum_id, accum));
                    }
                }
            }
        }

        if scrolled {
            ui.input_mut(|i| {
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                i.raw_scroll_delta = egui::Vec2::ZERO;
            });
        }

        // Keyboard navigation when hovered and not actively typing in a cell
        if edit_buffer.1.is_empty() {
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
                let step = if ctrl { 1024 } else { 256 };
                *base_addr = base_addr.wrapping_add(step) & 0x00FF_FFF0;
            } else if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
                let step = if ctrl { 1024 } else { 256 };
                *base_addr = base_addr.wrapping_sub(step) & 0x00FF_FFF0;
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                *base_addr = base_addr.wrapping_add(16) & 0x00FF_FFF0;
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                *base_addr = base_addr.wrapping_sub(16) & 0x00FF_FFF0;
            } else if ui.input(|i| i.key_pressed(egui::Key::Home)) {
                *base_addr = 0x000000;
            } else if ui.input(|i| i.key_pressed(egui::Key::End)) {
                *base_addr = 0x07FF00;
            }
        }
    }

    ui.heading("Memory Hex Editor");

    // 1. Fast Chunk Quick-Jump Buttons
    ui.horizontal_wrapped(|ui| {
        let chunks = [
            ("Vectors $000000", 0x000000),
            ("Low RAM $001000", 0x001000),
            ("Screen $070000", 0x070000),
            ("Slow RAM $C00000", 0xC00000),
            ("Kickstart $FC0000", 0xFC0000),
            ("Custom $DFF000", 0xDFF000),
            ("CIA-A $BFE001", 0xBFE001),
        ];

        for (label, addr) in chunks {
            if ui.button(label).clicked() {
                *base_addr = addr & 0x00FF_FFF0;
            }
        }
    });

    // Address navigation input
    ui.horizontal(|ui| {
        ui.label("Address:");
        let mut addr_str = format!("{:06X}", *base_addr);
        if ui
            .add(egui::TextEdit::singleline(&mut addr_str).desired_width(70.0))
            .lost_focus()
        {
            let clean = addr_str.trim().trim_start_matches('$');
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                *base_addr = addr & 0x00FF_FFF0;
            }
        }

        if ui.button("▲ Prev").clicked() {
            *base_addr = base_addr.wrapping_sub(256) & 0x00FF_FFF0;
        }
        if ui.button("▼ Next").clicked() {
            *base_addr = base_addr.wrapping_add(256) & 0x00FF_FFF0;
        }
    });

    ui.separator();

    // 2. 16-byte Hex + ASCII Grid with Infinite Scroll & Vertical Scrollbar
    const ROW_BYTES: usize = 16;
    let row_height = 18.0;
    const CELL_WIDTH: f32 = 18.5;
    let num_rows = ((ui.available_height() - 220.0) / row_height)
        .floor()
        .clamp(16.0, 36.0) as usize;
    let grid_height = (num_rows as f32) * row_height;

    let mut next_edit_target: Option<u32> = None;
    const SCROLLBAR_WIDTH: f32 = 10.0;
    let table_width = (ui.available_width() - SCROLLBAR_WIDTH - 8.0).max(100.0);

    ui.horizontal(|ui| {
        egui::ScrollArea::horizontal()
            .id_salt("hex_grid_hscroll")
            .max_width(table_width)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Grid Table Column
                ui.vertical(|ui| {
                    for row in 0..num_rows {
                        let row_addr =
                            base_addr.wrapping_add((row * ROW_BYTES) as u32) & 0x00FF_FFFF;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;

                            // Address header
                            ui.monospace(
                                RichText::new(format!("{:06X}:", row_addr))
                                    .color(tokens.address_blue),
                            );
                            ui.add_space(2.0);

                            // 16 Hex Bytes (each allocated with exact size so active editing can NEVER shift columns)
                            for col in 0..ROW_BYTES {
                                let byte_addr = row_addr.wrapping_add(col as u32) & 0x00FF_FFFF;
                                let byte_val = bus.read_byte_debug(byte_addr);
                                let active_wp = breakpoints.find_watchpoint_at(byte_addr).cloned();

                                // Calculate diff status from prev_memory
                                let is_changed = match prev_memory {
                                    Some((prev_bytes, prev_base)) if prev_base == *base_addr => {
                                        let offset = (row * ROW_BYTES) + col;
                                        offset < prev_bytes.len() && byte_val != prev_bytes[offset]
                                    }
                                    _ => false,
                                };

                                let (cell_rect, cell_response) = ui.allocate_exact_size(
                                    egui::vec2(CELL_WIDTH, row_height),
                                    egui::Sense::click(),
                                );

                                // If user is actively editing this byte
                                if edit_buffer.0 == byte_addr && !edit_buffer.1.is_empty() {
                                    ui.allocate_new_ui(
                                        egui::UiBuilder::new().max_rect(cell_rect),
                                        |ui| {
                                            let edit_res = ui.add(
                                                egui::TextEdit::singleline(&mut edit_buffer.1)
                                                    .margin(egui::Margin::ZERO)
                                                    .desired_width(CELL_WIDTH)
                                                    .font(egui::FontId::monospace(13.0)),
                                            );
                                            edit_res.request_focus();

                                            if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                                || edit_res.clicked_elsewhere()
                                            {
                                                edit_buffer.1.clear();
                                            } else if edit_res.lost_focus()
                                                || ui.input(|i| {
                                                    i.key_pressed(egui::Key::Tab)
                                                        || i.key_pressed(egui::Key::Enter)
                                                })
                                            {
                                                let clean = edit_buffer.1.trim();
                                                if let Ok(val) = u8::from_str_radix(clean, 16) {
                                                    bus.write_byte_debug(byte_addr, val);
                                                    if ui.input(|i| {
                                                        i.key_pressed(egui::Key::Tab)
                                                            || i.key_pressed(egui::Key::Enter)
                                                    }) {
                                                        next_edit_target = Some(
                                                            byte_addr.wrapping_add(1) & 0x00FF_FFFF,
                                                        );
                                                    }
                                                }
                                                edit_buffer.1.clear();
                                            }
                                        },
                                    );
                                } else {
                                    let text_color = if active_wp.is_some() {
                                        tokens.accent_error // Watched bytes
                                    } else if is_changed {
                                        tokens.accent_diff // Eye-friendly diff cyan
                                    } else if byte_val == 0 {
                                        tokens.text_muted // Muted slate gray for zeros
                                    } else {
                                        tokens.text_primary // Clean primary text
                                    };

                                    if active_wp.is_some() {
                                        // Crisp flat watchpoint indicator (0.0 corner radius, zero overlap)
                                        ui.painter().rect_filled(
                                            cell_rect,
                                            0.0,
                                            Color32::from_rgba_unmultiplied(
                                                tokens.accent_error.r(),
                                                tokens.accent_error.g(),
                                                tokens.accent_error.b(),
                                                60,
                                            ),
                                        );
                                        ui.painter().rect_stroke(
                                            cell_rect,
                                            0.0,
                                            egui::Stroke::new(1.0_f32, tokens.accent_error),
                                            egui::StrokeKind::Inside,
                                        );
                                    } else if cell_response.hovered() {
                                        ui.painter().rect_filled(
                                            cell_rect,
                                            0.0,
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                                        );
                                    }

                                    ui.painter().text(
                                        cell_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!("{:02X}", byte_val),
                                        egui::FontId::monospace(13.0),
                                        text_color,
                                    );

                                    // Tooltip for watchpoint or address
                                    if let Some(ref wp) = active_wp {
                                        cell_response.clone().on_hover_ui(|ui| {
                                            ui.heading("🎯 Active Memory Watchpoint");
                                            ui.label(format!("Address: ${:06X}", byte_addr));
                                            ui.label(format!(
                                                "Watch Range: ${:06X} .. ${:06X}",
                                                wp.start, wp.end
                                            ));
                                            ui.label(format!("Access Mode: {:?}", wp.access));
                                            ui.label(
                                        "Shortcuts: Alt+Click to toggle, Right-click for options",
                                    );
                                        });
                                    }

                                    // Context menu on right click
                                    cell_response.clone().context_menu(|ui| {
                                        ui.label(
                                            RichText::new(format!("Address: ${:06X}", byte_addr))
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.separator();
                                        if ui
                                            .button("🛑 Toggle Watchpoint Write (1 Byte)")
                                            .clicked()
                                        {
                                            breakpoints.toggle_byte_watchpoint(
                                                byte_addr,
                                                WatchAccess::Write,
                                            );
                                            ui.close_menu();
                                        }
                                        if ui.button("👁 Set Watchpoint Read (1 Byte)").clicked()
                                        {
                                            breakpoints.add_watchpoint(
                                                byte_addr,
                                                byte_addr,
                                                WatchAccess::Read,
                                            );
                                            ui.close_menu();
                                        }
                                        if ui
                                            .button("⚡ Set Watchpoint Any Access (1 Byte)")
                                            .clicked()
                                        {
                                            breakpoints.add_watchpoint(
                                                byte_addr,
                                                byte_addr,
                                                WatchAccess::Any,
                                            );
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui
                                            .button("Set Word Watchpoint (2 Bytes, Write)")
                                            .clicked()
                                        {
                                            breakpoints.add_watchpoint(
                                                byte_addr,
                                                byte_addr.wrapping_add(1) & 0x00FF_FFFF,
                                                WatchAccess::Write,
                                            );
                                            ui.close_menu();
                                        }
                                        if ui
                                            .button("Set Long Watchpoint (4 Bytes, Write)")
                                            .clicked()
                                        {
                                            breakpoints.add_watchpoint(
                                                byte_addr,
                                                byte_addr.wrapping_add(3) & 0x00FF_FFFF,
                                                WatchAccess::Write,
                                            );
                                            ui.close_menu();
                                        }
                                        if active_wp.is_some() {
                                            ui.separator();
                                            if ui.button("🗑 Remove Watchpoint").clicked() {
                                                breakpoints.remove_watchpoints_at(byte_addr);
                                                ui.close_menu();
                                            }
                                        }
                                    });

                                    if cell_response.clicked() {
                                        if ui.input(|i| i.modifiers.alt) {
                                            breakpoints.toggle_byte_watchpoint(
                                                byte_addr,
                                                WatchAccess::Write,
                                            );
                                        } else {
                                            edit_buffer.0 = byte_addr;
                                            edit_buffer.1 = format!("{:02X}", byte_val);
                                        }
                                    }
                                }

                                if col == 7 {
                                    ui.add_space(5.0); // Space between 8-byte halves
                                }
                            }

                            ui.add_space(3.0);
                            ui.monospace(RichText::new("|").color(tokens.border_subtle));
                            ui.add_space(3.0);

                            // ASCII representation with watchpoint highlights
                            let mut job = egui::text::LayoutJob::default();
                            for col in 0..ROW_BYTES {
                                let byte_addr = row_addr.wrapping_add(col as u32) & 0x00FF_FFFF;
                                let byte_val = bus.read_byte_debug(byte_addr);
                                let is_wp = breakpoints.find_watchpoint_at(byte_addr).is_some();
                                let ch = if byte_val.is_ascii_graphic() || byte_val == b' ' {
                                    byte_val as char
                                } else {
                                    '.'
                                };
                                let ch_color = if is_wp {
                                    tokens.accent_error
                                } else {
                                    tokens.text_secondary
                                };
                                job.append(
                                    &ch.to_string(),
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: ch_color,
                                        background: if is_wp {
                                            Color32::from_rgba_unmultiplied(
                                                tokens.accent_error.r(),
                                                tokens.accent_error.g(),
                                                tokens.accent_error.b(),
                                                80,
                                            )
                                        } else {
                                            Color32::TRANSPARENT
                                        },
                                        ..Default::default()
                                    },
                                );
                            }
                            ui.label(job);
                        });
                    }
                });
            });

        // Vertical scrollbar alongside table
        ui.add_space(4.0);
        render_vertical_scrollbar(ui, base_addr, grid_height, tokens);
    });

    if let Some(target_addr) = next_edit_target {
        edit_buffer.0 = target_addr;
        let val = bus.read_byte_debug(target_addr);
        edit_buffer.1 = format!("{:02X}", val);
    }

    let end_pos = ui.cursor().min;
    let total_rect = egui::Rect::from_min_max(
        start_pos,
        egui::pos2(ui.max_rect().max.x, end_pos.y.max(start_pos.y + 100.0)),
    );
    ui.data_mut(|d| d.insert_temp(mem_hex_rect_id, total_rect));
}

/// Interactive vertical scrollbar mapping 24-bit Amiga address space ($000000..=$00FFFFF0)
fn render_vertical_scrollbar(
    ui: &mut Ui,
    base_addr: &mut u32,
    track_height: f32,
    tokens: &ColorTokens,
) {
    const MAX_ADDR: f32 = 0x00FF_FFF0 as f32;
    const SCROLLBAR_WIDTH: f32 = 10.0;

    let (track_rect, track_response) = ui.allocate_exact_size(
        egui::vec2(SCROLLBAR_WIDTH, track_height),
        egui::Sense::click_and_drag(),
    );

    let thumb_height = 24.0_f32.max(track_height * 0.08);
    let fraction = (*base_addr as f32 / MAX_ADDR).clamp(0.0, 1.0);
    let travel = (track_rect.height() - thumb_height).max(1.0);
    let thumb_top = track_rect.top() + fraction * travel;
    let thumb_rect = egui::Rect::from_min_size(
        egui::pos2(track_rect.left() + 1.0, thumb_top),
        egui::vec2(track_rect.width() - 2.0, thumb_height),
    );

    // Track background
    ui.painter().rect_filled(track_rect, 2.0, tokens.card_bg);

    // Handle drag
    if track_response.dragged() {
        if let Some(ptr) = ui.input(|i| i.pointer.latest_pos()) {
            let rel_y = (ptr.y - track_rect.top() - thumb_height / 2.0).clamp(0.0, travel);
            let frac = rel_y / travel;
            *base_addr = ((frac * MAX_ADDR) as u32) & 0x00FF_FFF0;
        }
    } else if track_response.clicked() {
        if let Some(click_pos) = track_response.interact_pointer_pos() {
            if click_pos.y < thumb_rect.top() {
                *base_addr = base_addr.wrapping_sub(256) & 0x00FF_FFF0;
            } else if click_pos.y > thumb_rect.bottom() {
                *base_addr = base_addr.wrapping_add(256) & 0x00FF_FFF0;
            }
        }
    }

    // Thumb styling
    let thumb_color = if track_response.dragged() {
        tokens.border_active // Active drag
    } else if track_response.hovered() {
        tokens.text_secondary
    } else {
        tokens.border_subtle
    };

    ui.painter().rect_filled(thumb_rect, 2.0, thumb_color);

    // Tooltip showing current memory region
    let region = memory_region_name(*base_addr);
    track_response.on_hover_text(format!(
        "${:06X}: {}\nScroll: Wheel (16B), Shift+Wheel (256B), Ctrl+Wheel (1KB)",
        *base_addr, region
    ));
}

/// Helper mapping 24-bit addresses to descriptive Amiga hardware memory regions
fn memory_region_name(addr: u32) -> &'static str {
    match addr & 0x00FF_FFFF {
        0x000000..=0x07FFFF => "Chip RAM (512 KB)",
        0x080000..=0x0FFFFF => "Extended Chip RAM (0.5-1 MB)",
        0x100000..=0x1FFFFF => "Extended Chip RAM (1-2 MB)",
        0x200000..=0x9FFFFF => "Auto-Config Fast RAM (8 MB)",
        0xA00000..=0xBFFFFF => "CIA / Expansion / Reserved",
        0xC00000..=0xC7FFFF => "Slow / Pseudo-Fast RAM (512 KB)",
        0xC80000..=0xDDF000 => "Reserved / Real-Time Clock",
        0xDFF000..=0xDFFFFF => "Custom Chip Registers",
        0xE00000..=0xE7FFFF => "Reserved / CDTV ROM",
        0xE80000..=0xEFFFFF => "Auto-Config I/O Boards",
        0xF00000..=0xFBFFFF => "Reserved / Cartridge ROM",
        0xFC0000..=0xFFFFFF => "Kickstart ROM (256 KB)",
        _ => "Unmapped Space",
    }
}
