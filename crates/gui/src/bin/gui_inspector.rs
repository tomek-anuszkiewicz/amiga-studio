//! Vision-Driven Headless GUI Inspector & Frame Capture Tool
//!
//! Renders offscreen screenshots of the Amiga 500 Developer Studio (`EmulatorApp`)
//! at arbitrary resolutions using `egui_kittest` + `wgpu`. Supports automated
//! interaction scripting, splitter/scrollbar hover testing, inline editing,
//! and metadata JSON dumping for Agent Multimodal Vision inspection.

use std::fs;
use std::path::PathBuf;

use egui::{pos2, vec2, Event};
use egui_kittest::kittest::by;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use gui::{DisasmEditState, EditRegister, EmulatorApp};

fn print_usage() {
    eprintln!(
        "Usage: gui-inspector [OPTIONS]\n\
         Options:\n\
           --scenario <NAME>       Scenario: baseline, hover_splitter, hover_scrollbar, hover_register, hover_ccr, hover_memory, game_mode, workbench_theme, edit_register, edit_disasm, small_window (default: baseline)\n\
           --width <PIXELS>        Viewport width in points/pixels (default: 1280)\n\
           --height <PIXELS>       Viewport height in points/pixels (default: 720)\n\
           --output <PATH>         Target PNG output file path (default: target/gui_captures/baseline.png)\n\
           --json-metadata <PATH>  Optional file path to dump widget AccessKit bounds JSON\n\
           --help                  Show this help message\n"
    );
}

fn create_primed_app() -> EmulatorApp {
    let mut app = EmulatorApp::default();

    // Load sample assembly program: NOP, NOP, MOVE.W #$1234, D0, RTS
    let code: [u8; 10] = [
        0x4E, 0x71, // NOP
        0x4E, 0x71, // NOP
        0x30, 0x3C, 0x12, 0x34, // MOVE.W #$1234, D0
        0x4E, 0x75, // RTS
    ];
    app.session.load_binary(0x001000, &code, true);

    // Seed realistic register and debugger state
    app.session.machine.cpu.state.set_d_long(0, 0xCAFE_BABE);
    app.session.machine.cpu.state.set_d_long(1, 0x0000_1000);
    app.session.machine.cpu.state.set_a_long(0, 0x0000_2000);
    app.session.machine.cpu.state.set_sr(0x2700);

    // Step one instruction so trace log has entries and PC moves to $1002
    app.session.step_instruction();

    app
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut scenario = "baseline".to_string();
    let mut width: f32 = 1280.0;
    let mut height: f32 = 720.0;
    let mut output_path = PathBuf::from("target/gui_captures/baseline.png");
    let mut json_metadata_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                if i < args.len() {
                    scenario = args[i].clone();
                }
            }
            "--width" => {
                i += 1;
                if i < args.len() {
                    width = args[i].parse().unwrap_or(1280.0);
                }
            }
            "--height" => {
                i += 1;
                if i < args.len() {
                    height = args[i].parse().unwrap_or(720.0);
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = PathBuf::from(&args[i]);
                }
            }
            "--json-metadata" => {
                i += 1;
                if i < args.len() {
                    json_metadata_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            unknown => {
                eprintln!("Unknown option: {}", unknown);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if scenario == "small_window" && width == 1280.0 && height == 720.0 {
        width = 1024.0;
        height = 600.0;
    }

    println!(
        "🎨 [gui-inspector] Running scenario: '{}' at resolution {}x{}",
        scenario, width, height
    );

    let app = if scenario == "clean_startup" {
        EmulatorApp::default()
    } else {
        create_primed_app()
    };

    // Prepare harness with wgpu offscreen renderer
    let mut harness = Harness::builder()
        .with_size(vec2(width, height))
        .wgpu()
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0);
                app.update_ui(ctx);
            },
            app,
        );

    // Initial render step to lay out widgets and build AccessKit tree
    harness.step();

    // Apply scenario-specific interactions
    match scenario.as_str() {
        "baseline" | "clean_startup" => {
            // Settle layout with another step
            harness.step();
        }
        "hover_splitter" => {
            // Left dock is default 320 wide, right dock is at the right edge
            // Hover mouse at the left dock resize border (x ~= 320)
            harness
                .input_mut()
                .events
                .push(Event::PointerMoved(pos2(320.0, 300.0)));
            harness.step();
        }
        "hover_scrollbar" => {
            // Hover over the right dock scrollbar region
            harness
                .input_mut()
                .events
                .push(Event::PointerMoved(pos2(width - 6.0, 200.0)));
            harness.step();
        }
        "hover_register" => {
            // Hover mouse over D0 in Left Dock to verify register documentation tooltip
            harness
                .input_mut()
                .events
                .push(Event::PointerMoved(pos2(40.0, 95.0)));
            harness.step();
        }
        "hover_ccr" => {
            // Hover mouse over Condition Code Register (CCR) flags
            harness
                .input_mut()
                .events
                .push(Event::PointerMoved(pos2(215.0, 290.0)));
            harness.step();
        }
        "hover_memory" => {
            // Hover mouse over Memory Hex scrollbar to verify memory region map tooltip
            harness
                .input_mut()
                .events
                .push(Event::PointerMoved(pos2(width - 25.0, 300.0)));
            harness.step();
        }
        "game_mode" => {
            // Standalone ScreenOnly mode with retro CRT framing
            harness.state_mut().view_mode = gui::ViewMode::ScreenOnly;
            harness.step();
        }
        "workbench_theme" => {
            // Classic Amiga Workbench color palette
            harness.state_mut().theme = gui::AppTheme::ClassicWorkbench;
            harness.step();
        }
        "edit_register" => {
            // Trigger inline edit on D0
            harness.state_mut().active_reg_edit =
                Some((EditRegister::D(0), "DEADBEEF".to_string()));
            harness.step();
        }
        "edit_disasm" => {
            // Trigger inline edit on disassembly at $1000
            harness.state_mut().active_disasm_edit = Some(DisasmEditState {
                addr: 0x001000,
                text: "MOVE.W D0, D1".to_string(),
                error: None,
            });
            harness.step();
        }
        "small_window" => {
            // Layout with constrained size
            harness.step();
        }
        other => {
            eprintln!(
                "⚠️ Unrecognized scenario '{}', capturing baseline state.",
                other
            );
            harness.step();
        }
    }

    // Capture final offscreen image
    let image = harness
        .render()
        .map_err(|e| format!("Offscreen render failed: {}", e))?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut opaque_image = image;
    for pixel in opaque_image.pixels_mut() {
        if pixel[3] < 255 {
            let a = pixel[3] as f32 / 255.0;
            let bg_r = 18.0;
            let bg_g = 20.0;
            let bg_b = 24.0;
            pixel[0] = ((pixel[0] as f32 * a) + bg_r * (1.0 - a)) as u8;
            pixel[1] = ((pixel[1] as f32 * a) + bg_g * (1.0 - a)) as u8;
            pixel[2] = ((pixel[2] as f32 * a) + bg_b * (1.0 - a)) as u8;
            pixel[3] = 255;
        }
    }

    opaque_image.save(&output_path)?;
    println!(
        "✅ Frame successfully captured and saved to: {}",
        output_path.display()
    );

    // Optional AccessKit JSON metadata dump
    if let Some(meta_path) = json_metadata_path {
        if let Some(parent) = meta_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let nodes = harness.node().query_all(by().recursive(true));
        let mut json_nodes = Vec::new();
        for node in nodes {
            let label = node.label().unwrap_or_default();
            let role = format!("{:?}", node.role());
            let rect = node.bounding_box();
            json_nodes.push(format!(
                "  {{\"role\": \"{}\", \"label\": \"{}\", \"rect\": {:?}}}",
                role, label, rect
            ));
        }
        let json_str = format!("[\n{}\n]", json_nodes.join(",\n"));
        fs::write(&meta_path, json_str)?;
        println!("📄 AccessKit metadata saved to: {}", meta_path.display());
    }

    Ok(())
}
