//! Right Dock: Memory Search Component
//!
//! Pattern search in memory using Hex sequences or ASCII strings.

use egui::{Color32, RichText, Ui};
use physical_memory::PhysicalMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    HexSequence,
    AsciiString,
}

#[derive(Debug, Clone, Default)]
pub struct MemorySearchState {
    pub query: String,
    pub mode: SearchMode,
    pub status_message: Option<(String, bool)>, // (message, is_error)
}

pub(crate) fn render_memory_search(
    ui: &mut Ui,
    bus: &PhysicalMemory,
    base_addr: &mut u32,
    state: &mut MemorySearchState,
) {
    ui.spacing_mut().indent = 0.0;
    egui::CollapsingHeader::new(RichText::new("🔍 Memory Search").strong())
        .default_open(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.mode,
                    SearchMode::HexSequence,
                    RichText::new("Hex Sequence").strong(),
                );
                ui.selectable_value(
                    &mut state.mode,
                    SearchMode::AsciiString,
                    RichText::new("ASCII String").strong(),
                );
            });

            ui.horizontal(|ui| {
                let hint = match state.mode {
                    SearchMode::HexSequence => "e.g. 4E 71 32 00",
                    SearchMode::AsciiString => "e.g. DOS or AMIGA",
                };
                let btn_width = 95.0;
                let text_width =
                    (ui.available_width() - btn_width - ui.spacing().item_spacing.x).max(80.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text(hint)
                        .desired_width(text_width),
                );

                if ui
                    .add_sized([btn_width, 20.0], egui::Button::new("🔍 Find Next"))
                    .clicked()
                {
                    find_next(bus, base_addr, state);
                }
            });

            if let Some((msg, is_err)) = &state.status_message {
                let color = if *is_err {
                    Color32::from_rgb(255, 100, 100)
                } else {
                    Color32::from_rgb(100, 220, 120)
                };
                ui.colored_label(color, msg);
            }
        });
}

fn parse_query_bytes(query: &str, mode: SearchMode) -> Result<Vec<u8>, String> {
    match mode {
        SearchMode::AsciiString => {
            if query.is_empty() {
                Err("Empty search query".to_string())
            } else {
                Ok(query.as_bytes().to_vec())
            }
        }
        SearchMode::HexSequence => {
            let mut bytes = Vec::new();
            let tokens = query.split_whitespace();
            for token in tokens {
                let clean = token.trim_start_matches('$');
                match u8::from_str_radix(clean, 16) {
                    Ok(b) => bytes.push(b),
                    Err(_) => return Err(format!("Invalid hex byte '{}'", token)),
                }
            }
            if bytes.is_empty() {
                Err("Empty hex query".to_string())
            } else {
                Ok(bytes)
            }
        }
    }
}

fn find_next(bus: &PhysicalMemory, base_addr: &mut u32, state: &mut MemorySearchState) {
    let pattern = match parse_query_bytes(&state.query, state.mode) {
        Ok(p) => p,
        Err(err) => {
            state.status_message = Some((err, true));
            return;
        }
    };

    let start = base_addr.wrapping_add(1) & 0x00FF_FFFF;
    let search_limit = 512 * 1024; // Scan up to 512 KB

    for offset in 0..search_limit {
        let candidate_addr = start.wrapping_add(offset as u32) & 0x00FF_FFFF;
        let mut matched = true;

        for (i, &expected_byte) in pattern.iter().enumerate() {
            let addr = candidate_addr.wrapping_add(i as u32) & 0x00FF_FFFF;
            if bus.read_byte_debug(addr) != expected_byte {
                matched = false;
                break;
            }
        }

        if matched {
            *base_addr = candidate_addr & 0x00FF_FFF0;
            state.status_message = Some((format!("Match found at ${:06X}", candidate_addr), false));
            return;
        }
    }

    state.status_message = Some(("Pattern not found in memory".to_string(), true));
}
