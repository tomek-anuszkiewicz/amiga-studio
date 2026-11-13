//! Right Dock: Memory Hex Editor Component
//!
//! 16-byte hex + ASCII viewer and editor with fast chunk navigation buttons,
//! infinite mouse wheel scrolling, keyboard navigation, vertical interactive scrollbar,
//! inline interactive editing, and step diff highlighting.

use crate::theme::ColorTokens;
use debugger::{BreakpointManager, WatchAccess};
use egui::{Color32, RichText, Ui};
use physical_memory::PhysicalMemory;

const CHIP_RAM_LAST_PAGE: u32 = CHIP_RAM_LAST & !(PAGE_BYTES - 1);
const LOW_RAM_SHORTCUT: u32 = 0x001000;
const SCREEN_RAM_SHORTCUT: u32 = 0x070000;

const ROW_BYTES: usize = 16;
const MASK_ROW_BYTE: u32 = ROW_BYTES as u32 - 1;
const ROW_HEIGHT: f32 = 18.0;
const MIN_NAV_ROWS: f32 = 6.0;
const MAX_NAV_ROWS: f32 = 48.0;
const PAGE_BYTES: u32 = 256;
const CONTROL_PAGE_BYTES: u32 = 1024;
const FAST_SCROLL_ROWS: u32 = 256;
const CONTROL_SCROLL_ROWS: u32 = 64;
const SHIFT_SCROLL_ROWS: u32 = 16;
const ALT_SCROLL_ROWS: u32 = 4;
const DEFAULT_SCROLL_ROWS: u32 = 1;
const CHIP_RAM_FIRST: u32 = 0x000000;
const CHIP_RAM_LAST: u32 = 0x07FFFF;
const CHIP_HALF_TO_ONE_MB_FIRST: u32 = 0x080000;
const CHIP_HALF_TO_ONE_MB_LAST: u32 = 0x0FFFFF;
const CHIP_ONE_TO_TWO_MB_FIRST: u32 = 0x100000;
const CHIP_ONE_TO_TWO_MB_LAST: u32 = 0x1FFFFF;
const FAST_EXPANSION_FIRST: u32 = 0x200000;
const FAST_EXPANSION_LAST: u32 = 0x9FFFFF;
const CIA_EXPANSION_FIRST: u32 = 0xA00000;
const CIA_EXPANSION_LAST: u32 = 0xBFFFFF;
const SLOW_RAM_FIRST: u32 = 0xC00000;
const SLOW_RAM_LAST: u32 = 0xC7FFFF;
const RESERVED_RTC_FIRST: u32 = 0xC80000;
const RESERVED_RTC_LAST: u32 = 0xDDF000;
const CUSTOM_REGISTERS_FIRST: u32 = 0xDFF000;
const CUSTOM_REGISTERS_LAST: u32 = 0xDFFFFF;
const CDTV_ROM_FIRST: u32 = 0xE00000;
const CDTV_ROM_LAST: u32 = 0xE7FFFF;
const AUTOCONFIG_IO_FIRST: u32 = 0xE80000;
const AUTOCONFIG_IO_LAST: u32 = 0xEFFFFF;
const CARTRIDGE_ROM_FIRST: u32 = 0xF00000;
const CARTRIDGE_ROM_LAST: u32 = 0xFBFFFF;
const KICKSTART_ROM_FIRST: u32 = 0xFC0000;
const KICKSTART_ROM_LAST: u32 = 0xFFFFFF;

pub const MASK_ROW_ADDRESS: u32 = physical_memory::MASK_PHYSICAL_ADDRESS & !MASK_ROW_BYTE;

#[derive(Debug, Default)]
pub(crate) struct MemoryPresentation {
    address: String,
    scrollbar_tooltip: String,
    rows: Vec<MemoryRow>,
}

#[derive(Debug)]
struct MemoryByte {
    address: u32,
    value: u8,
    hex: String,
    changed: bool,
    watchpoint: bool,
    tooltip: Option<String>,
}

#[derive(Debug)]
struct MemoryRow {
    address_label: String,
    bytes: [MemoryByte; ROW_BYTES],
    ascii: egui::text::LayoutJob,
}

impl MemoryPresentation {
    fn refresh(
        &mut self,
        bus: &PhysicalMemory,
        breakpoints: &BreakpointManager,
        base: u32,
        count: usize,
        selected: Option<u32>,
        previous: Option<(&[u8; debugger::session::MEMORY_DIFF_BYTES], u32)>,
        tokens: &ColorTokens,
    ) {
        self.rows.clear();
        self.scrollbar_tooltip = format!(
            "${base:06X}: {}\nScroll: Wheel (16B), Shift+Wheel (256B), Ctrl+Wheel (1KB)",
            memory_region_name(base)
        );
        for row in 0..count {
            let address = base.wrapping_add((row * ROW_BYTES) as u32)
                & physical_memory::MASK_PHYSICAL_ADDRESS;
            let mut ascii = egui::text::LayoutJob::default();
            let bytes = std::array::from_fn(|col| {
                let address =
                    address.wrapping_add(col as u32) & physical_memory::MASK_PHYSICAL_ADDRESS;
                let value = bus.read_byte_debug(address);
                let wp = breakpoints.find_watchpoint_at(address);
                let is_selected = selected == Some(address);
                let color = if wp.is_some() {
                    tokens.accent_error
                } else if is_selected {
                    tokens.border_active
                } else {
                    tokens.text_secondary
                };
                let background = if wp.is_some() || is_selected {
                    Color32::from_rgba_unmultiplied(
                        color.r(),
                        color.g(),
                        color.b(),
                        if wp.is_some() { 80 } else { 60 },
                    )
                } else {
                    Color32::TRANSPARENT
                };
                let ch = if value.is_ascii_graphic() || value == b' ' {
                    value as char
                } else {
                    '.'
                };
                ascii.append(
                    &ch.to_string(),
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color,
                        background,
                        ..Default::default()
                    },
                );
                let offset = row * ROW_BYTES + col;
                MemoryByte {
                    address, value, hex: format!("{value:02X}"),
                    changed: previous.is_some_and(|(bytes, old_base)| old_base == base && bytes.get(offset).is_some_and(|old| *old != value)),
                    watchpoint: wp.is_some(),
                    tooltip: wp.map(|wp| format!("Address: ${address:06X}\nWatch Range: ${:06X} .. ${:06X}\nAccess Mode: {:?}", wp.start, wp.end, wp.access)),
                }
            });
            self.rows.push(MemoryRow {
                address_label: format!("{address:06X}:"),
                bytes,
                ascii,
            });
        }
    }
}

pub(crate) fn render_memory_hex(
    ui: &mut Ui,
    data: &mut MemoryPresentation,
    refresh: bool,
    interactive: bool,
    bus: &mut PhysicalMemory,
    breakpoints: &mut BreakpointManager,
    base_addr: &mut u32,
    selected_addr: &mut Option<u32>,
    edit_buffer: &mut (u32, String),
    prev_memory: Option<(&[u8; debugger::session::MEMORY_DIFF_BYTES], u32)>,
    tokens: &ColorTokens,
) {
    let row_height = ROW_HEIGHT;
    let start_pos = ui.cursor().min;
    let mem_hex_rect_id = egui::Id::new("mem_hex_rect");
    let prev_rect: Option<egui::Rect> = ui.data(|d| d.get_temp(mem_hex_rect_id));
    let is_hovered = prev_rect.map_or(false, |r| ui.rect_contains_pointer(r));

    // Calculate approximate visible rows for keyboard navigation bounds
    let nav_num_rows = ((ui.available_height() - 90.0) / row_height)
        .floor()
        .clamp(MIN_NAV_ROWS, MAX_NAV_ROWS) as usize;
    let nav_visible_bytes = (nav_num_rows * ROW_BYTES) as u32;

    // 0. Mouse Wheel Infinite Scroll & Keyboard Navigation Handling
    if is_hovered && ui.is_enabled() {
        let mut scrolled = false;

        for event in ui.input(|i| i.events.clone()) {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } = event
            {
                let multiplier: u32 = if modifiers.ctrl && modifiers.shift {
                    FAST_SCROLL_ROWS // 4 KB per notch
                } else if modifiers.ctrl || modifiers.command {
                    CONTROL_SCROLL_ROWS // 1 KB per notch
                } else if modifiers.shift {
                    SHIFT_SCROLL_ROWS // 256 bytes (1 full page) per notch
                } else if modifiers.alt {
                    ALT_SCROLL_ROWS // 64 bytes per notch
                } else {
                    DEFAULT_SCROLL_ROWS // 1 row (16 bytes) per notch
                };

                match unit {
                    egui::MouseWheelUnit::Line => {
                        let clicks = (-delta.y.round()) as i32;
                        if clicks > 0 {
                            *base_addr = base_addr
                                .wrapping_add((clicks as u32) * multiplier * ROW_BYTES as u32)
                                & MASK_ROW_ADDRESS;
                            scrolled = true;
                        } else if clicks < 0 {
                            *base_addr = base_addr
                                .wrapping_sub(((-clicks) as u32) * multiplier * ROW_BYTES as u32)
                                & MASK_ROW_ADDRESS;
                            scrolled = true;
                        }
                    }
                    egui::MouseWheelUnit::Point | egui::MouseWheelUnit::Page => {
                        let accum_id = ui.id().with("hex_scroll_accum");
                        let mut accum: f32 = ui.data(|d| d.get_temp(accum_id)).unwrap_or(0.0);
                        accum += delta.y;
                        const POINTS_PER_ROW: f32 = ROW_HEIGHT;
                        if accum <= -POINTS_PER_ROW {
                            let rows = (-accum / POINTS_PER_ROW) as u32;
                            accum += (rows as f32) * POINTS_PER_ROW;
                            *base_addr = base_addr
                                .wrapping_add(rows * multiplier * ROW_BYTES as u32)
                                & MASK_ROW_ADDRESS;
                            scrolled = true;
                        } else if accum >= POINTS_PER_ROW {
                            let rows = (accum / POINTS_PER_ROW) as u32;
                            accum -= (rows as f32) * POINTS_PER_ROW;
                            *base_addr = base_addr
                                .wrapping_sub(rows * multiplier * ROW_BYTES as u32)
                                & MASK_ROW_ADDRESS;
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

            if let Some(sel) = *selected_addr {
                let visible_end = base_addr.wrapping_add(nav_visible_bytes)
                    & physical_memory::MASK_PHYSICAL_ADDRESS;

                if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    let new_sel = sel.wrapping_sub(1) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    if new_sel < *base_addr || new_sel >= visible_end {
                        *base_addr = new_sel & MASK_ROW_ADDRESS;
                    }
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    let new_sel = sel.wrapping_add(1) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    if new_sel >= visible_end || new_sel < *base_addr {
                        *base_addr = new_sel
                            .wrapping_sub(nav_visible_bytes.saturating_sub(ROW_BYTES as u32))
                            & MASK_ROW_ADDRESS;
                    }
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    let new_sel =
                        sel.wrapping_sub(ROW_BYTES as u32) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    if new_sel < *base_addr || new_sel >= visible_end {
                        *base_addr = new_sel & MASK_ROW_ADDRESS;
                    }
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    let new_sel =
                        sel.wrapping_add(ROW_BYTES as u32) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    if new_sel >= visible_end || new_sel < *base_addr {
                        *base_addr = base_addr.wrapping_add(ROW_BYTES as u32) & MASK_ROW_ADDRESS;
                    }
                } else if ui.input(|i| i.key_pressed(egui::Key::Home)) {
                    let new_sel = sel & !MASK_ROW_BYTE;
                    *selected_addr = Some(new_sel);
                } else if ui.input(|i| i.key_pressed(egui::Key::End)) {
                    let new_sel = (sel & !MASK_ROW_BYTE) | MASK_ROW_BYTE;
                    *selected_addr = Some(new_sel);
                } else if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
                    let step = if ctrl {
                        CONTROL_PAGE_BYTES
                    } else {
                        nav_visible_bytes.max(PAGE_BYTES)
                    };
                    let new_sel = sel.wrapping_sub(step) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    *base_addr = base_addr.wrapping_sub(step) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
                    let step = if ctrl {
                        CONTROL_PAGE_BYTES
                    } else {
                        nav_visible_bytes.max(PAGE_BYTES)
                    };
                    let new_sel = sel.wrapping_add(step) & physical_memory::MASK_PHYSICAL_ADDRESS;
                    *selected_addr = Some(new_sel);
                    *base_addr = base_addr.wrapping_add(step) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let val = bus.read_byte_debug(sel);
                    edit_buffer.0 = sel;
                    edit_buffer.1 = format!("{:02X}", val);
                    let just_started_id = egui::Id::new("hex_just_started_edit");
                    ui.data_mut(|d| d.insert_temp(just_started_id, true));
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *selected_addr = None;
                }
            } else {
                if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
                    let step = if ctrl { CONTROL_PAGE_BYTES } else { PAGE_BYTES };
                    *base_addr = base_addr.wrapping_add(step) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
                    let step = if ctrl { CONTROL_PAGE_BYTES } else { PAGE_BYTES };
                    *base_addr = base_addr.wrapping_sub(step) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    *base_addr = base_addr.wrapping_add(ROW_BYTES as u32) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    *base_addr = base_addr.wrapping_sub(ROW_BYTES as u32) & MASK_ROW_ADDRESS;
                } else if ui.input(|i| i.key_pressed(egui::Key::Home)) {
                    *base_addr = cpu::vector::addr(cpu::vector::RESET_SSP);
                } else if ui.input(|i| i.key_pressed(egui::Key::End)) {
                    *base_addr = CHIP_RAM_LAST_PAGE;
                }
            }
        }
    }

    ui.heading("Memory Hex Editor");

    // 1. Fast Chunk Quick-Jump Buttons
    ui.horizontal_wrapped(|ui| {
        let chunks = [
            ("Vectors $000000", cpu::vector::addr(cpu::vector::RESET_SSP)),
            ("Low RAM $001000", LOW_RAM_SHORTCUT),
            ("Screen $070000", SCREEN_RAM_SHORTCUT),
            ("Slow RAM $C00000", SLOW_RAM_FIRST),
            ("Kickstart $FC0000", KICKSTART_ROM_FIRST),
            ("Custom $DFF000", CUSTOM_REGISTERS_FIRST),
            ("CIA-A $BFE001", config::cia_reg::cia_a::REGISTER_PRA),
        ];

        for (label, addr) in chunks {
            if ui.button(label).clicked() {
                *base_addr = addr & MASK_ROW_ADDRESS;
            }
        }
    });

    // Address navigation input
    ui.horizontal(|ui| {
        ui.label("Address:");
        if refresh {
            data.address = format!("{:06X}", *base_addr);
        }
        if ui
            .add(egui::TextEdit::singleline(&mut data.address).desired_width(70.0))
            .lost_focus()
            && ui.is_enabled()
        {
            let clean = data.address.trim().trim_start_matches('$');
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                *base_addr = addr & MASK_ROW_ADDRESS;
            }
        }
    });

    ui.separator();

    // 2. 16-byte Hex + ASCII Grid with Infinite Scroll & Vertical Scrollbar
    const CELL_WIDTH: f32 = 18.5;
    let num_rows = ((ui.available_height() - 8.0) / row_height)
        .floor()
        .clamp(4.0, 48.0) as usize;
    if refresh {
        data.refresh(
            bus,
            breakpoints,
            *base_addr,
            num_rows,
            *selected_addr,
            prev_memory,
            tokens,
        );
    }
    let grid_height = (data.rows.len() as f32) * row_height;

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
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for row in &data.rows {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;

                            // Address header
                            ui.monospace(
                                RichText::new(&row.address_label)
                                    .color(tokens.address_blue),
                            );
                            ui.add_space(2.0);

                            // 16 Hex Bytes (each allocated with exact size so active editing can NEVER shift columns)
                            for col in 0..ROW_BYTES {
                                let byte = &row.bytes[col];
                                let byte_addr = byte.address;
                                let byte_val = byte.value;
                                let is_selected = *selected_addr == Some(byte_addr);
                                let is_changed = byte.changed;

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
                                            if interactive { edit_res.request_focus(); }

                                            let just_started_id =
                                                egui::Id::new("hex_just_started_edit");
                                            let just_started: bool = ui
                                                .data(|d| d.get_temp(just_started_id))
                                                .unwrap_or(false);

                                            let should_commit_edit = || {
                                                interactive && !just_started
                                                    && (edit_res.lost_focus()
                                                        || ui.input(|i| {
                                                            i.key_pressed(egui::Key::Tab)
                                                                || i.key_pressed(egui::Key::Enter)
                                                        }))
                                            };
                                            if interactive && (ui.input(|i| i.key_pressed(egui::Key::Escape))
                                                || edit_res.clicked_elsewhere())
                                            {
                                                edit_buffer.1.clear();
                                            } else if should_commit_edit() {
                                                let clean = edit_buffer.1.trim();
                                                if let Ok(val) = u8::from_str_radix(clean, 16) {
                                                    bus.write_byte_debug(byte_addr, val);
                                                    if ui.input(|i| {
                                                        i.key_pressed(egui::Key::Tab)
                                                            || i.key_pressed(egui::Key::Enter)
                                                    }) {
                                                        next_edit_target = Some(
                                                            byte_addr.wrapping_add(1) & physical_memory::MASK_PHYSICAL_ADDRESS,
                                                        );
                                                    }
                                                }
                                                edit_buffer.1.clear();
                                            }
                                        },
                                    );
                                } else {
                                    let text_color = if byte.watchpoint {
                                        tokens.accent_error // Watched bytes
                                    } else if is_changed {
                                        tokens.accent_diff // Eye-friendly diff cyan
                                    } else if byte_val == 0 {
                                        tokens.text_muted // Muted slate gray for zeros
                                    } else {
                                        tokens.text_primary // Clean primary text
                                    };

                                    if byte.watchpoint {
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
                                    } else if is_selected {
                                        ui.painter().rect_filled(
                                            cell_rect,
                                            0.0,
                                            Color32::from_rgba_unmultiplied(
                                                tokens.border_active.r(),
                                                tokens.border_active.g(),
                                                tokens.border_active.b(),
                                                45,
                                            ),
                                        );
                                        ui.painter().rect_stroke(
                                            cell_rect,
                                            0.0,
                                            egui::Stroke::new(1.5_f32, tokens.border_active),
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
                                        &byte.hex,
                                        egui::FontId::monospace(13.0),
                                        text_color,
                                    );

                                    // Tooltip for watchpoint or address
                                    if let Some(tooltip) = &byte.tooltip {
                                        cell_response.clone().on_hover_ui(|ui| {
                                            ui.heading("🎯 Active Memory Watchpoint");
                                            ui.label(tooltip);
                                            ui.label("Shortcuts: Alt+Click to toggle, Right-click for options");
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
                                                byte_addr.wrapping_add(1) & physical_memory::MASK_PHYSICAL_ADDRESS,
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
                                                byte_addr.wrapping_add(3) & physical_memory::MASK_PHYSICAL_ADDRESS,
                                                WatchAccess::Write,
                                            );
                                            ui.close_menu();
                                        }
                                        if byte.watchpoint {
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
                                            *selected_addr = Some(byte_addr);
                                            edit_buffer.1.clear();
                                        }
                                    }
                                    if cell_response.double_clicked() {
                                        *selected_addr = Some(byte_addr);
                                        edit_buffer.0 = byte_addr;
                                        edit_buffer.1 = format!("{:02X}", byte_val);
                                    }
                                }

                                if col == ROW_BYTES / 2 - 1 {
                                    ui.add_space(5.0); // Space between 8-byte halves
                                }
                            }

                            ui.add_space(3.0);
                            ui.monospace(RichText::new("|").color(tokens.border_subtle));
                            ui.add_space(3.0);

                            // ASCII representation with watchpoint and selection highlights
                            ui.label(row.ascii.clone());
                        });
                    }
                });
            });

        // Vertical scrollbar alongside table
        ui.add_space(4.0);
        render_vertical_scrollbar(ui, base_addr, grid_height, tokens, &data.scrollbar_tooltip);
    });

    if let Some(target_addr) = next_edit_target {
        *selected_addr = Some(target_addr);
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
    tooltip: &str,
) {
    const MAX_ADDR: f32 = MASK_ROW_ADDRESS as f32;
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
            *base_addr = ((frac * MAX_ADDR) as u32) & MASK_ROW_ADDRESS;
        }
    } else if track_response.clicked() {
        if let Some(click_pos) = track_response.interact_pointer_pos() {
            if click_pos.y < thumb_rect.top() {
                *base_addr = base_addr.wrapping_sub(PAGE_BYTES) & MASK_ROW_ADDRESS;
            } else if click_pos.y > thumb_rect.bottom() {
                *base_addr = base_addr.wrapping_add(PAGE_BYTES) & MASK_ROW_ADDRESS;
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

    track_response.on_hover_text(tooltip);
}

/// Helper mapping 24-bit addresses to descriptive Amiga hardware memory regions
fn memory_region_name(addr: u32) -> &'static str {
    match addr & physical_memory::MASK_PHYSICAL_ADDRESS {
        CHIP_RAM_FIRST..=CHIP_RAM_LAST => "Chip RAM (512 KB)",
        CHIP_HALF_TO_ONE_MB_FIRST..=CHIP_HALF_TO_ONE_MB_LAST => "Extended Chip RAM (0.5-1 MB)",
        CHIP_ONE_TO_TWO_MB_FIRST..=CHIP_ONE_TO_TWO_MB_LAST => "Extended Chip RAM (1-2 MB)",
        FAST_EXPANSION_FIRST..=FAST_EXPANSION_LAST => "Auto-Config Fast RAM (8 MB)",
        CIA_EXPANSION_FIRST..=CIA_EXPANSION_LAST => "CIA / Expansion / Reserved",
        SLOW_RAM_FIRST..=SLOW_RAM_LAST => "Slow / Pseudo-Fast RAM (512 KB)",
        RESERVED_RTC_FIRST..=RESERVED_RTC_LAST => "Reserved / Real-Time Clock",
        CUSTOM_REGISTERS_FIRST..=CUSTOM_REGISTERS_LAST => "Custom Chip Registers",
        CDTV_ROM_FIRST..=CDTV_ROM_LAST => "Reserved / CDTV ROM",
        AUTOCONFIG_IO_FIRST..=AUTOCONFIG_IO_LAST => "Auto-Config I/O Boards",
        CARTRIDGE_ROM_FIRST..=CARTRIDGE_ROM_LAST => "Reserved / Cartridge ROM",
        KICKSTART_ROM_FIRST..=KICKSTART_ROM_LAST => "Kickstart ROM (256 KB)",
        _ => "Unmapped Space",
    }
}
