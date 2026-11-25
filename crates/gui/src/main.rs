//! Native Desktop Entrypoint for Amiga 500 Developer Studio

#[cfg(not(target_arch = "wasm32"))]
use gui::{app::APPLICATION_TITLE, EmulatorApp, ViewMode};

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
            .with_title(APPLICATION_TITLE)
            .with_app_id("amiga-studio")
            .with_inner_size([1600.0, 840.0])
            .with_min_inner_size([1024.0, 600.0]),
        ..Default::default()
    };

    #[cfg(feature = "game-fps-benchmark")]
    let game_fps = match (
        std::env::var_os("AMIGA_GAME_FPS_ROM"),
        std::env::var_os("AMIGA_GAME_FPS_ADF"),
        std::env::var_os("AMIGA_GAME_FPS_RESULT"),
    ) {
        (None, None, None) => None,
        (Some(rom), Some(adf), Some(result)) => Some((
            std::path::PathBuf::from(rom),
            std::path::PathBuf::from(adf),
            std::path::PathBuf::from(result),
        )),
        _ => return Err("Game FPS requires ROM, ADF and result paths".into()),
    };
    #[cfg(feature = "game-fps-benchmark")]
    let native_options = if let Some((_, _, result)) = &game_fps {
        eframe::NativeOptions {
            persist_window: false,
            persistence_path: Some(result.with_extension("ron")),
            ..native_options
        }
    } else {
        native_options
    };

    gui::window_keyboard::run_native(
        APPLICATION_TITLE,
        native_options,
        Box::new(move |cc| {
            let mut app = EmulatorApp::new(cc);
            if let Some(mode) = force_mode {
                app.view_mode = mode;
            }
            #[cfg(feature = "game-fps-benchmark")]
            if let Some((rom, adf, result)) = game_fps {
                gui::game_fps::attach_native(&mut app, cc, &rom, &adf, result);
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
