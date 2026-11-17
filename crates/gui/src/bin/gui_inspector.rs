//! Vision-Driven Headless GUI Inspector & Frame Capture Tool
//!
//! Renders offscreen screenshots of the Amiga 500 Developer Studio (`EmulatorApp`)
//! at arbitrary resolutions using `egui_kittest` + `wgpu`. Supports automated
//! interaction scripting, splitter/scrollbar hover testing, inline editing,
//! and metadata JSON dumping for Agent Multimodal Vision inspection.

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use egui::{pos2, vec2, Event};
#[cfg(not(target_arch = "wasm32"))]
use egui_kittest::kittest::by;
#[cfg(not(target_arch = "wasm32"))]
use egui_kittest::kittest::Queryable;
#[cfg(not(target_arch = "wasm32"))]
use egui_kittest::Harness;
#[cfg(not(target_arch = "wasm32"))]
use gui::{DisasmEditState, EditRegister, EmulatorApp};

#[cfg(not(target_arch = "wasm32"))]
fn print_usage() {
    eprintln!(
        "Usage: gui-inspector [OPTIONS]\n\
         Options:\n\
           --scenario <NAME>       Scenario: baseline, clean_startup, live_display, live_display_studio, mixed_scene, hover_splitter, hover_scrollbar, hover_register, hover_ccr, hover_memory, game_mode, drive_activity, library, library_editor, manual_setup, replace_session, configurations, workbench_theme, edit_register, edit_disasm, small_window (default: baseline)\n\
           --width <PIXELS>        Viewport width in points/pixels (default: 1280)\n\
           --height <PIXELS>       Viewport height in points/pixels (default: 720)\n\
           --output <PATH>         Target PNG output file path (default: target/gui_captures/baseline.png)\n\
           --json-metadata <PATH>  Optional file path to dump widget AccessKit bounds JSON\n\
           --help                  Show this help message\n"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn create_primed_app() -> Result<EmulatorApp, physical_memory::KickstartSizeError> {
    let mut session = Box::new(gui::DebuggerSession::new());

    // Load sample assembly program: NOP, NOP, MOVE.W #$1234, D0, RTS
    let code: [u8; 10] = [
        0x4E, 0x71, // NOP
        0x4E, 0x71, // NOP
        0x30, 0x3C, 0x12, 0x34, // MOVE.W #$1234, D0
        0x4E, 0x75, // RTS
    ];
    session.load_binary(debugger::DEFAULT_TARGET_ADDRESS, &code, true)?;

    // Seed realistic register and debugger state
    session.machine.cpu.state.set_d_long(0, 0xCAFE_BABE);
    session.machine.cpu.state.set_d_long(1, 0x0000_1000);
    session.machine.cpu.state.set_a_long(0, 0x0000_2000);
    session.machine.cpu.state.set_sr(0x2700);

    // Step one instruction so trace log has entries and PC moves to $1002
    session.step_instruction();

    Ok(EmulatorApp::with_session(session))
}

#[cfg(not(target_arch = "wasm32"))]
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

    let app = if scenario == "mixed_scene" {
        let mut session = Box::new(gui::DebuggerSession::new());
        session.load_binary(
            debugger::DEFAULT_TARGET_ADDRESS,
            include_bytes!("../../../machine_loop/tests/programs/display/mixed_scene/program.bin"),
            true,
        )?;
        // Advance the real guest past initialization before capturing its image.
        for _ in 0..64 {
            session.machine.step_frame();
        }
        let mut app = EmulatorApp::with_session(session);
        app.view_mode = gui::ViewMode::ScreenOnly;
        app
    } else if matches!(scenario.as_str(), "live_display" | "live_display_studio") {
        let mut session = Box::new(gui::DebuggerSession::new());
        session.load_binary(
            debugger::DEFAULT_TARGET_ADDRESS,
            include_bytes!("../../../machine_loop/tests/programs/display/live_stripes/program.bin"),
            true,
        )?;
        let mut app = EmulatorApp::with_session(session);
        if scenario == "live_display" {
            app.view_mode = gui::ViewMode::ScreenOnly;
        }
        app.host.queue(gui::host_commands::HostCommand::Run);
        app
    } else if scenario == "drive_activity" {
        const IDLE_GUEST_CODE: [u8; 4] = [0x4e, 0x71, 0x60, 0xfc];
        const RELEASED_CONTROLS: u8 = u8::MAX;
        const MOTOR_READY_CONTROLS: u8 = 0x7f;
        const RUNNING_DF0_CONTROLS: u8 = 0x77;
        const CIA_CAPTURE_CCKS: u64 = 5;
        const SUSTAINED_DRIVE_CCKS: u64 = 2000;
        let mut session = Box::new(gui::DebuggerSession::new());
        session.load_binary(debugger::DEFAULT_TARGET_ADDRESS, &IDLE_GUEST_CODE, true)?;
        // Commit generic CIA registers, then deliver the connector normally.
        session.machine.cia_b.set_prb(RELEASED_CONTROLS);
        session.machine.cia_b.set_ddrb(RELEASED_CONTROLS);
        session.machine.step_cycles(CIA_CAPTURE_CCKS);
        session.machine.cia_b.set_prb(MOTOR_READY_CONTROLS);
        session.machine.step_cycles(CIA_CAPTURE_CCKS);
        session.machine.cia_b.set_prb(RUNNING_DF0_CONTROLS);
        session.machine.step_cycles(SUSTAINED_DRIVE_CCKS);
        let mut app = EmulatorApp::with_session(session);
        app.view_mode = gui::ViewMode::ScreenOnly;
        app
    } else if scenario == "clean_startup" {
        EmulatorApp::default()
    } else {
        create_primed_app()?
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
    let inspection_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        harness.step();
        if harness.state().host.is_idle() {
            break;
        }
        if std::time::Instant::now() >= inspection_deadline {
            return Err("GUI initialization did not settle".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    // Apply scenario-specific interactions
    match scenario.as_str() {
        "live_display" | "live_display_studio" => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while harness.state().host.status.field_number < 2 {
                if std::time::Instant::now() >= deadline {
                    return Err("Display program did not produce two completed fields".into());
                }
                harness.step();
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            harness
                .state_mut()
                .host
                .queue(gui::host_commands::HostCommand::Pause);
            loop {
                harness.step();
                if !harness.state().host.status.running && harness.state().host.is_idle() {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    return Err("Display program did not pause".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        "library" | "library_editor" | "manual_setup" | "replace_session" | "configurations" => {
            let screen = match scenario.as_str() {
                "library" | "library_editor" => gui::player::PlayerScreen::Library,
                "manual_setup" | "replace_session" => gui::player::PlayerScreen::Setup,
                _ => gui::player::PlayerScreen::Configuration,
            };
            harness.state_mut().navigate_player(screen);
            if scenario == "library_editor" {
                let mut draft = gui::library_edit::GameDraft::new("standard".into());
                draft.name = "Example game".into();
                draft.description = "Optional notes about the game and its disks.".into();
                harness.state_mut().player.catalog.library_root = "GameLibrary".into();
                harness.state_mut().player.library_editor = Some(draft);
            }
            if scenario == "replace_session" {
                let app = harness.state_mut();
                app.player.active_configuration = Some(Default::default());
                app.player.catalog.configurations[0].rom = "preview.rom".into();
                app.player.rom = Some(vec![0; physical_memory::KICKSTART_ROM_SIZE]);
                app.player.rom_configuration_id = Some("standard".into());
                app.player.rom_reference = Some("preview.rom".into());
                harness.step();
                harness.step();
                harness.get_by_label("Start").click();
                harness.step();
                harness.step();
                harness.get_by_label("Replace session");
            }
            harness.step();
        }
        "baseline" | "clean_startup" | "mixed_scene" => {
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
        "game_mode" | "drive_activity" => {
            // Standalone ScreenOnly mode with retro CRT framing
            harness.state_mut().view_mode = gui::ViewMode::ScreenOnly;
            harness.step();
            if width < 850.0 {
                // Capture the expanded controls without an intermediate animation frame.
                harness.ctx.style_mut(|style| style.animation_time = 0.0);
                harness.get_by_label("Game controls").click();
                harness.step();
                harness.step();
            }
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
                addr: debugger::DEFAULT_TARGET_ADDRESS,
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

// Trunk builds package targets; the browser application lives in the GUI library.
#[cfg(target_arch = "wasm32")]
fn main() {}
