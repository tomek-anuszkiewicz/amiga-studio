//! Native Desktop Entrypoint for Amiga 500 Developer Studio

#[cfg(not(target_arch = "wasm32"))]
use gui::{EmulatorApp, ViewMode};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut force_mode: Option<ViewMode> = None;
    let mut load_file: Option<String> = None;
    let mut load_addr: u32 = 0x001000;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--game" | "--screen-only" => force_mode = Some(ViewMode::ScreenOnly),
            "--dev" | "--debugger" => force_mode = Some(ViewMode::Developer),
            "--load" => {
                if i + 1 < args.len() {
                    load_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--addr" => {
                if i + 1 < args.len() {
                    let clean = args[i + 1]
                        .trim()
                        .trim_start_matches('$')
                        .trim_start_matches("0x");
                    if let Ok(addr) = u32::from_str_radix(clean, 16) {
                        load_addr = addr;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Amiga 500 Developer Studio")
            .with_inner_size([1600.0, 840.0])
            .with_min_inner_size([1024.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Amiga 500 Developer Studio",
        native_options,
        Box::new(move |cc| {
            let mut app = EmulatorApp::new(cc);
            if let Some(mode) = force_mode {
                app.view_mode = mode;
            }
            if let Some(path_str) = load_file {
                if let Ok(data) = std::fs::read(&path_str) {
                    app.session.load_binary(load_addr, &data, true);
                    app.hex_base_addr = load_addr & 0x00FF_FFF0;
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
