//! Native Desktop Entrypoint for Amiga 500 Developer Studio

#[cfg(not(target_arch = "wasm32"))]
use gui::{EmulatorApp, ViewMode};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut force_mode = None;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--game" | "--screen-only" => force_mode = Some(ViewMode::ScreenOnly),
            "--dev" | "--debugger" => force_mode = Some(ViewMode::Developer),
            "--help" | "-h" => {
                println!("Usage: amiga-studio [--game | --dev]\n  --screen-only is an alias for --game.\n  --debugger is an alias for --dev.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option {argument:?}. Use --help for usage.").into()),
        }
    }

    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::WgpuConfiguration {
            surface: eframe::SurfaceConfig {
                present_mode: eframe::wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: Some(2),
            },
            ..Default::default()
        },
        viewport: egui::ViewportBuilder::default()
            .with_title("Amiga 500 Developer Studio")
            .with_app_id("amiga-studio")
            .with_inner_size([1600.0, 840.0])
            .with_min_inner_size([1024.0, 600.0]),
        ..Default::default()
    };

    gui::window_keyboard::run_native(
        "Amiga 500 Developer Studio",
        native_options,
        Box::new(move |cc| {
            let mut app = EmulatorApp::new(cc);
            if let Some(mode) = force_mode {
                app.view_mode = mode;
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
