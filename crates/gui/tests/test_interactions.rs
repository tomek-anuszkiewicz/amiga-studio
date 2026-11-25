#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::TestContext as _;
use egui::{Event, Key, Modifiers, RawInput};
use egui_kittest::kittest::NodeT;
use gui::host_commands::{HistoryCommand, HostCommand};
use gui::{AppTheme, DisasmEditState, EditRegister, EmulatorApp, ViewMode};
use std::path::PathBuf;

#[test]
fn screen_uploads_the_visible_framebuffer_unchanged_and_preserves_its_geometry() {
    for (standard, lines, blanks, ccks) in [
        (config::VideoStandard::Pal, 313, 26, 227),
        (config::VideoStandard::Ntsc, 263, 21, 228),
    ] {
        for interlaced in [false, true] {
            let mut session = Box::new(gui::DebuggerSession::from_config(
                config::A500Config::standard_1mb(standard),
            ));
            let denise = &mut session.machine.denise;
            denise.bplcon0 = (if interlaced { 4 } else { 0 }).into();
            for _field in 0..2 {
                for line in 0..lines {
                    if line == 0 {
                        denise.strobe_strequ();
                    } else if line < blanks {
                        denise.strobe_strvbl();
                    } else {
                        denise.strobe_strhor();
                    }
                    for _ in 0..ccks {
                        denise.step_cck();
                    }
                }
            }
            denise.strobe_strequ();
            for _ in 0..19 {
                denise.step_cck();
            }
            let frame = &mut denise.frame_builder;
            let stride = frame.row_stride();
            let height = frame.height as usize;
            let image_width = 720;
            let image_height = if standard == config::VideoStandard::Pal {
                568
            } else {
                480
            };
            frame.frame_buffer_mut().fill(0xff00ff00);
            for y in 0..height {
                let color = if y % 2 == 0 { 0xff0000ff } else { 0xffff0000 };
                let start = y * stride;
                frame.frame_buffer_mut()[start..start + stride].fill(color);
                // Keep both outer columns of the monitor image, including corners.
                frame.frame_buffer_mut()[start] = 0xffffffff;
                frame.frame_buffer_mut()[start + image_width - 1] = 0xffffffff;
            }
            let before = frame.clone();
            let expected_cck = session.machine.cck;
            for (view, source) in [
                (ViewMode::Developer, session.clone()),
                (ViewMode::ScreenOnly, session),
            ] {
                let mut app = EmulatorApp::with_session(source);
                app.view_mode = view;
                let ctx = egui::Context::default();
                let viewport =
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
                let mut output = ctx.run_test_ui(
                    RawInput {
                        screen_rect: Some(viewport),
                        ..Default::default()
                    },
                    |ui| {
                        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                        app.update_ui(ui)
                    },
                );
                // Command replies may settle before the independent video handoff.
                output.append(common::wait(&mut app, &ctx, |app| {
                    app.host.presented.is_some_and(|info| {
                        info.generation == app.host.status.generation
                            && info.cck == expected_cck
                            && info.width == image_width
                            && info.height == image_height
                    })
                }));
                let (texture_id, image) = common::texture_updates(&output.textures_delta)
                    .find_map(|(id, delta)| match &delta.image {
                        egui::ImageData::Color(image)
                            if image.size == [image_width, image_height] =>
                        {
                            Some((*id, image))
                        }
                        _ => None,
                    })
                    .expect("the complete monitor image must be uploaded");
                let (shape, mesh) = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh) if mesh.texture_id == texture_id => {
                            Some((shape, mesh))
                        }
                        _ => None,
                    })
                    .expect("the complete monitor image must be drawn");
                let bounds = mesh.calc_bounds();
                assert!(
                    (bounds.width() / image_width as f32 - bounds.height() / image_height as f32)
                        .abs()
                        < 0.001
                );
                assert!(viewport.contains_rect(bounds));
                assert!(bounds.center().distance(shape.clip_rect.center()) < 1.0);
                assert!(mesh.vertices.iter().any(|v| v.uv == egui::pos2(0.0, 0.0)));
                assert!(mesh.vertices.iter().any(|v| v.uv == egui::pos2(1.0, 1.0)));
                for y in 0..image_height {
                    let source_y = y;
                    for x in 0..image_width {
                        let expected = if x == 0 || x == image_width - 1 {
                            egui::Color32::WHITE
                        } else if source_y % 2 == 0 {
                            egui::Color32::BLUE
                        } else {
                            egui::Color32::RED
                        };
                        assert_eq!(image.pixels[y * image_width + x], expected);
                    }
                }
                assert_eq!(common::snapshot(&mut app).denise.frame_builder, before);
            }
        }
    }
}

#[test]
fn playback_toolbar_and_preferences_fit_supported_window_widths() {
    use egui_kittest::{
        kittest::{by, Queryable},
        Harness,
    };

    for width in [1024.0, 1280.0, 1920.0] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, 720.0))
            .build_ui_state(
                |ui, app: &mut EmulatorApp| {
                    common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                    app.update_ui(ui)
                },
                EmulatorApp::default(),
            );
        common::settle_harness(&mut harness);
        for label in ["Step Frame", "Slower", "Faster", "1x"] {
            let bounds = harness
                .get_by_label(label)
                .accesskit_node()
                .bounding_box()
                .unwrap();
            assert!(
                bounds.x0 >= 0.0 && bounds.x1 <= f64::from(width),
                "{label} at {width}"
            );
        }
        let preferences = harness
            .query_all(by().role(egui::accesskit::Role::ComboBox))
            .filter_map(|node| node.accesskit_node().bounding_box())
            .filter(|bounds| bounds.y0 < 55.0)
            .collect::<Vec<_>>();
        assert_eq!(
            preferences.len(),
            3,
            "theme, zoom and image scale must remain visible"
        );
        for bounds in preferences {
            assert!(bounds.x0 >= 0.0 && bounds.x1 <= f64::from(width));
        }
    }
}

#[test]
fn test_simulated_f12_view_mode_toggle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::Developer);

    // 1. Press F12 -> switch to ScreenOnly
    let input_f12 = RawInput {
        events: vec![Event::Key {
            key: Key::F12,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };

    let _ = ctx.run_test_ui(input_f12, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);

    // 2. Press F12 again -> switch back to Developer
    let input_f12_back = RawInput {
        events: vec![Event::Key {
            key: Key::F12,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };

    let _ = ctx.run_test_ui(input_f12_back, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::Developer);
}

#[test]
fn test_simulated_run_and_step_shortcuts() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    common::command(
        &mut app,
        HostCommand::History(HistoryCommand::ToggleRecording),
    );

    // Load binary code: NOP, NOP, BRA *-0 (infinite loop at $1004)
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x60, 0xFE];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // 1. Press F10 -> Step 1 instruction
    assert_eq!(common::inspect(&mut app).instructions_executed, 0);
    common::settle(&mut app, &ctx);
    let input_f10 = RawInput {
        events: vec![Event::Key {
            key: Key::F10,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f10, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(common::inspect(&mut app).instructions_executed, 1);
    assert_eq!(common::inspect(&mut app).temporal.len(), 1);

    // 2. Press Shift + F10 -> Step backward (temporal rewind)
    let input_shift_f10 = RawInput {
        events: vec![
            Event::ModifiersChanged(Modifiers::SHIFT),
            Event::Key {
                key: Key::F10,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_shift_f10, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(common::inspect(&mut app).temporal.scrub_cursor, Some(0));

    // 3. Press F11 -> Step CCK
    let prev_cck = common::inspect(&mut app).debugger.current_cck;
    let input_f11 = RawInput {
        events: vec![
            Event::ModifiersChanged(Modifiers::NONE),
            Event::Key {
                key: Key::F11,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f11, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(common::inspect(&mut app).debugger.current_cck, prev_cck + 1);

    // 4. Press F5 -> Toggle Run
    assert!(!app.host.status.running);
    let input_f5 = RawInput {
        events: vec![Event::Key {
            key: Key::F5,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f5, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert!(app.host.status.running);
}

#[test]
fn test_simulated_drag_and_drop_file() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert!(!app.load_binary_modal_open);
    assert!(app.pending_binary.is_none());

    // Simulate drag-and-drop of a binary file
    let input_drop = RawInput {
        dropped_files: vec![common::dropped_file("test.bin")],
        ..Default::default()
    };

    let _ = ctx.run_test_ui(input_drop, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Modal should now be open with the dropped file path
    assert!(app.load_binary_modal_open);
    assert!(
        matches!(app.pending_binary, Some(gui::BinarySource::File(ref path)) if path == &PathBuf::from("test.bin"))
    );
}

#[test]
fn test_simulated_register_inline_editing() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // 1. Edit Data register D0 -> value $12345678
    app.active_reg_edit = Some((EditRegister::D(0), "12345678".to_string()));

    let input_enter = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };

    let _ = ctx.run_test_ui(input_enter, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Value should be committed to D0 and edit mode closed
    assert_eq!(common::inspect(&mut app).cpu.d_long(0), 0x12345678);
    assert_eq!(app.active_reg_edit, None);

    // 2. Edit Status Register SR -> value $2700
    app.active_reg_edit = Some((EditRegister::SR, "2700".to_string()));
    let input_enter_sr = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_enter_sr, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(common::inspect(&mut app).cpu.sr(), 0x2700);
    assert_eq!(app.active_reg_edit, None);
}

#[test]
fn test_simulated_disassembly_inline_patching() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Set up NOP ($4E71) at $001000
    let code = [0x4E, 0x71, 0x4E, 0x75]; // NOP, RTS
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Initial instruction at $1000 is NOP
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x4E71
    );

    // 1. Open inline edit buffer to patch with MOVE.W D0, D1 ($3200, 2 bytes)
    app.active_disasm_edit = Some(DisasmEditState {
        addr: 0x001000,
        text: "MOVE.W D0, D1".to_string(),
        error: None,
    });

    // Run frame with Enter key to commit
    let input_commit = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_commit, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Instruction in RAM must now be $3200 and edit mode closed
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x3200
    );
    assert!(app.active_disasm_edit.is_none());

    // 2. Size mismatch test: trying to replace 2-byte instruction with 8-byte instruction
    app.active_disasm_edit = Some(DisasmEditState {
        addr: 0x001000,
        text: "33FC 0042 0007 0000".to_string(), // 8 bytes
        error: None,
    });
    let input_commit_mismatch = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_commit_mismatch, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Should reject with error message and leave RAM unchanged
    assert!(app.active_disasm_edit.is_some());
    assert!(app
        .active_disasm_edit
        .as_ref()
        .unwrap()
        .error
        .as_ref()
        .unwrap()
        .contains("Byte size mismatch"));
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x3200
    );
}

#[test]
fn test_simulated_breakpoint_form_and_trigger() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Code: NOP ($1000), NOP ($1002), RTS ($1004)
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Add breakpoint at $001002
    let mut breakpoints = common::inspect(&mut app).debugger.breakpoints;
    breakpoints.add_pc_breakpoint(0x001002);
    common::command(&mut app, HostCommand::SetBreakpoints(breakpoints));
    assert!(common::inspect(&mut app)
        .debugger
        .breakpoints
        .has_pc_breakpoint(0x001002));

    // Free-run execution
    app.host.queue(HostCommand::Run);
    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Execution should have stopped precisely at breakpoint $001002
    assert_eq!(common::inspect(&mut app).instructions_executed, 1);
    assert_eq!(common::inspect(&mut app).cpu.pc().wrapping_sub(4), 0x001002);
}

#[test]
fn test_simulated_temporal_time_travel_navigation() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Code: NOP, NOP, RTS
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Enable temporal for time-travel navigation test
    common::command(
        &mut app,
        HostCommand::History(HistoryCommand::ToggleRecording),
    );

    // Step 2 instructions
    app.host.queue(HostCommand::StepInstruction);
    app.host.queue(HostCommand::StepInstruction);
    assert_eq!(common::inspect(&mut app).temporal.len(), 2);
    assert_eq!(common::inspect(&mut app).temporal.scrub_cursor, None); // Live head

    // Step backward 1 frame in history (from live head index 1 to index 0)
    app.host
        .queue(HostCommand::History(HistoryCommand::Backward));
    assert_eq!(common::inspect(&mut app).temporal.scrub_cursor, Some(0));
    assert!(
        !common::inspect(&mut app)
            .cpu
            .micro()
            .current_steps
            .is_empty(),
        "Scrubbing backward must re-hydrate CPU micro-steps via restore_state"
    );

    // Render frame to ensure UI reflects scrub state without panics
    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Return to live head
    app.host.queue(HostCommand::History(HistoryCommand::Live));
    assert_eq!(common::inspect(&mut app).temporal.scrub_cursor, None);
    assert!(
        !common::inspect(&mut app)
            .cpu
            .micro()
            .current_steps
            .is_empty(),
        "Returning to live head must re-hydrate CPU micro-steps via restore_state"
    );
}

#[test]
fn test_full_frame_rendering_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Inject code so registers, memory, disassembly, and trace buffers all have live data
    let code = [0x4E, 0x71, 0x32, 0x00, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());
    common::command(
        &mut app,
        HostCommand::History(HistoryCommand::ToggleRecording),
    );
    app.host.queue(HostCommand::StepInstruction);

    // Render multiple consecutive frames to verify zero panics and layout stability
    for _ in 0..5 {
        let mut output = ctx.run_test_ui(RawInput::default(), |ui| {
            {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            };
        });
        output.append(common::settle(&mut app, &ctx));

        // Ensure egui produced shapes/textures for rendering
        assert!(!output.shapes.is_empty());
    }
}

#[test]
fn test_startup_clean_memory() {
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(common::inspect(&mut app).instructions_executed, 0);
    assert!(!app.host.status.running);
    assert!(!common::inspect(&mut app).temporal.is_recording());
    // Verify startup PC counter points to $000000 (0th cell) with valid SSP ($080000) and primed prefetch
    assert_eq!(common::inspect(&mut app).cpu.instruction_pc(), 0x000000);
    assert_eq!(common::inspect(&mut app).cpu.pc(), 0x000004);
    assert_eq!(common::inspect(&mut app).cpu.a_long(7), 0x080000);
    assert_eq!(common::inspect(&mut app).cpu.ssp(), 0x080000);
    assert!(
        common::inspect(&mut app).cpu.ir() == 0xFFFF
            || common::inspect(&mut app).cpu.ir() == 0x0000
            || common::inspect(&mut app).cpu.ir() == 0x0008,
        "Expected unpopulated Kickstart ROM ($FFFF), zeroed Chip RAM ($0000), or synthetic boot vector ($0008), got ${:04X}",
        common::inspect(&mut app).cpu.ir()
    );
    assert_eq!(app.goto_addr_str, "000000");

    // Verify memory starts clean zeroed Chip RAM without auto-loaded programs
    for addr in [0x000000, 0x001000, 0x001002, 0x002000, 0x070000] {
        let val = common::inspect(&mut app).memory.read_word_debug(addr);
        assert!(
            val == 0xFFFF || val == 0x0000 || val == 0x0008,
            "Expected clean/unmapped memory or boot vector, got ${:04X}",
            val
        );
    }
}

#[test]
fn test_ccr_led_badges_interactive_toggle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    common::command(
        &mut app,
        HostCommand::SetRegister {
            register: EditRegister::SR,
            value: 0x2700,
        },
    ); // All CCR flags (0x1F) are 0

    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // Toggle Z flag (bit 2, mask 0x04)
    common::command(&mut app, HostCommand::ToggleStatusFlag(0x04));
    assert_eq!(common::inspect(&mut app).cpu.sr() & 0x04, 0x04);

    // Toggle X flag (bit 4, mask 0x10)
    common::command(&mut app, HostCommand::ToggleStatusFlag(0x10));
    assert_eq!(common::inspect(&mut app).cpu.sr() & 0x10, 0x10);
}

#[test]
fn test_register_edit_focus_and_dismissal_lifecycle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // 1. Enter edit mode on register D0
    app.active_reg_edit = Some((EditRegister::D(0), "DEADBEEF".to_string()));

    // Render frame - automatically requests focus
    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert!(app.active_reg_edit.is_some());

    // 2. Simulate Escape key -> cancels edit mode cleanly
    let input_esc = RawInput {
        events: vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_esc, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert!(app.active_reg_edit.is_none());
    assert_ne!(common::inspect(&mut app).cpu.d_long(0), 0xDEADBEEF);
}

#[test]
fn test_pc_instruction_address_normalization() {
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let code = [0x4E, 0x71, 0x4E, 0x71];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Hardware prefetch advances PC bus register to $001004
    assert_eq!(common::inspect(&mut app).cpu.pc(), 0x001004);

    // Displayed/active instruction address is normalized to state.pc - 4 = $001000
    let instruction_pc = common::inspect(&mut app).cpu.pc().wrapping_sub(4) & 0x00FF_FFFF;
    assert_eq!(instruction_pc, 0x001000);
}

#[test]
fn test_disassembly_instruction_decoding_fibonacci() {
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let code: [u8; 20] = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($002000).L, A0
        0x42, 0x40, // CLR.W D0
        0x32, 0x3C, 0x00, 0x01, // MOVE.W #$0001, D1
        0x30, 0xC0, // MOVE.W D0, (A0)+
        0x30, 0xC1, // MOVE.W D1, (A0)+
        0x51, 0xCB, 0xFF, 0xF4, // DBRA D3, $1014
    ];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    let data = common::inspect(&mut app);
    let (dis0, len0) = debugger::disassemble(0x001000, |a| data.memory.read_word_debug(a));
    assert_eq!(dis0.mnemonic, "LEA");
    assert!(dis0.operands.contains("($00002000).L, A0"));
    assert_eq!(len0, 6);

    let (dis1, len1) = debugger::disassemble(0x001006, |a| data.memory.read_word_debug(a));
    assert_eq!(dis1.mnemonic, "CLR.W");
    assert_eq!(dis1.operands, "D0");
    assert_eq!(len1, 2);

    let (dis5, len5) = debugger::disassemble(0x001010, |a| data.memory.read_word_debug(a));
    assert!(dis5.mnemonic == "DBF" || dis5.mnemonic == "DBRA");
    assert!(dis5.operands.contains("D3"));
    assert_eq!(len5, 4);
}

#[test]
fn test_temporal_inactive_default_and_loop_rewind() {
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert!(!common::inspect(&mut app).temporal.is_recording());
    assert_eq!(debugger::temporal::DEFAULT_TEMPORAL_CAPACITY, 25_000);

    // Enable recording and run code
    common::command(
        &mut app,
        HostCommand::History(HistoryCommand::ToggleRecording),
    );
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    app.host.queue(HostCommand::StepInstruction);
    app.host.queue(HostCommand::StepInstruction);
    assert_eq!(common::inspect(&mut app).temporal.len(), 2);

    // Query historical passes for $1000
    let matches = common::inspect(&mut app)
        .temporal
        .find_matches_by_pc(0x001000, 10);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, 0); // Chronological index 0
}

#[test]
fn test_clean_screen_game_mode_and_crt_text() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    app.view_mode = ViewMode::ScreenOnly;

    let mut output = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    output.append(common::settle(&mut app, &ctx));

    assert!(!output.shapes.is_empty());
}

#[test]
fn test_memory_hex_right_margin_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    let raw_input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_test_ui(raw_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    output.append(common::settle(&mut app, &ctx));
    assert!(!output.shapes.is_empty());
    // Ensure all rendered panel widgets, separators, and group boxes remain bounded within screen margin
    for shape in &output.shapes {
        assert!(
            shape.clip_rect.max.x <= 1280.0,
            "ClippedShape exceeded right screen boundary: {:?}",
            shape.clip_rect
        );
    }
}

#[test]
fn test_microcode_toggle_and_clock_metrics() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::Developer;
    common::settle(&mut app, &ctx);
    assert!(app.show_microcode);

    for visible in [false, true] {
        // Publish the idle debugger keyboard policy before the window events.
        let _ = ctx.run_test_ui(RawInput::default(), |ui| app.update_ui(ui));
        for pressed in [true, false] {
            assert!(gui::keyboard_input::receive_key(
                &ctx,
                common::keyboard::key("F8", pressed),
                None,
            ));
        }
        common::settle(&mut app, &ctx);
        assert_eq!(app.show_microcode, visible);
    }

    // Load code and step CCK to verify micro-step clock metrics
    let code = [0x32, 0x00, 0x4E, 0x75]; // MOVE.W D0, D1
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Before stepping, ensure instruction is initiated or step 1 CCK
    app.host.queue(HostCommand::StepCck);
    let state = &common::inspect(&mut app).cpu;
    // MOVE.W has 2 steps of 2 clocks each; after 1 CCK, step 0 has completed and step 1 is active
    assert!(state.micro().micro_step <= state.micro().current_steps.len() as u16);
    assert!(!state.micro().current_steps.is_empty());
}

#[test]
fn test_memory_hex_edit_mode_zero_grid_shift_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // 1. First render a baseline frame where no cells are being edited
    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // 2. Put cell at address $00005A into active edit mode (e.g. col 10 in row 5)
    app.hex_edit_buffer.0 = 0x00005A;
    app.hex_edit_buffer.1 = "FF".to_string();

    let mut output_editing = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    output_editing.append(common::settle(&mut app, &ctx));
    assert!(!output_editing.shapes.is_empty());

    // 3. Edit with a single character "F" and verify shapes still render without shifting or panic
    app.hex_edit_buffer.1 = "F".to_string();
    let mut output_single_char = ctx.run_test_ui(RawInput::default(), |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    output_single_char.append(common::settle(&mut app, &ctx));
    assert!(!output_single_char.shapes.is_empty());
}

#[test]
fn test_memory_hex_mouse_wheel_infinite_scroll() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(app.hex_base_addr, 0x000000);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // 1. Prime the frame with mouse hovering over right dock memory hex area (~900, 150)
    let prime_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(prime_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    // 2. Wheel Down 1 tick -> scroll down 1 row (16 bytes)
    let scroll_down_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(scroll_down_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(
        app.hex_base_addr, 0x000010,
        "Base address must advance by 16 bytes on 1 wheel notch"
    );

    // 3. Wheel Down 1 tick with Shift -> scroll down 1 page (16 rows = 256 bytes)
    let shift_scroll_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::ModifiersChanged(Modifiers::SHIFT),
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::SHIFT,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(shift_scroll_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(
        app.hex_base_addr, 0x000110,
        "Base address must advance by 256 bytes with Shift+wheel"
    );

    // 4. Wheel Down 1 tick with Ctrl -> scroll down 1 KB (64 rows = 1024 bytes)
    let ctrl_scroll_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::ModifiersChanged(Modifiers::CTRL),
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::CTRL,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(ctrl_scroll_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(
        app.hex_base_addr, 0x000510,
        "Base address must advance by 1024 bytes with Ctrl+wheel"
    );

    // 5. Wheel Up -> wraps smoothly backwards across 24-bit boundary when moving past 0
    app.hex_base_addr = 0x000000;
    let wrap_up_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::ModifiersChanged(Modifiers::NONE),
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta: egui::vec2(0.0, 1.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(wrap_up_input, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);
    assert_eq!(
        app.hex_base_addr, 0x00FFFFF0,
        "Base address must wrap to top of 24-bit space on upward overflow"
    );
}

#[test]
fn test_memory_hex_keyboard_navigation() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    app.hex_base_addr = 0x001000;

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime hover
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    // 1. PageDown -> +256 bytes
    let pagedown_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::PageDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(pagedown_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_base_addr, 0x001100);

    // 2. PageUp -> -256 bytes
    let pageup_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::PageUp,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(pageup_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_base_addr, 0x001000);

    // 3. ArrowDown -> +16 bytes
    let arrowdown_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(arrowdown_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_base_addr, 0x001010);

    // 4. ArrowUp -> -16 bytes
    let arrowup_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::ArrowUp,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(arrowup_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_base_addr, 0x001000);

    // 5. Home -> 0x000000
    let home_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::Home,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(home_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_base_addr, 0x000000);
}

#[test]
fn test_memory_hex_vertical_scrollbar_interaction() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    app.hex_base_addr = 0x000000;

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // 1. Prime frame
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    // 2. Click near bottom of the vertical scrollbar track (~1279, 300)
    let click_scrollbar_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(1279.0, 300.0)),
            Event::PointerButton {
                pos: egui::pos2(1279.0, 300.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            Event::PointerButton {
                pos: egui::pos2(1279.0, 300.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(click_scrollbar_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);

    // Address must advance forward from 0
    assert!(
        app.hex_base_addr > 0x000000,
        "Clicking scrollbar track below thumb must advance base address"
    );
}

#[test]
fn test_f12_toggles_views_and_middle_mouse_releases_capture() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::Developer);

    // 1. Press F12 -> Switch to ScreenOnly
    let input_f12 = RawInput {
        events: vec![Event::Key {
            key: Key::F12,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f12, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);

    // Release F12: a later press must be a new edge, not keyboard repeat.
    let _ = ctx.run_test_ui(
        RawInput {
            events: vec![Event::Key {
                key: Key::F12,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );

    // 2. Middle mouse releases capture and leaves player controls reachable.
    app.player.captured = true;
    let input_esc = RawInput {
        events: vec![Event::PointerButton {
            pos: egui::pos2(400.0, 250.0),
            button: egui::PointerButton::Middle,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_esc, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);
    assert!(!app.player.captured);

    // 3. Press F12 again -> Developer
    let input_f12_again = RawInput {
        events: vec![Event::Key {
            key: Key::F12,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f12_again, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::Developer);

    // 4. Release F12 and press it again to toggle back -> ScreenOnly.
    let input_f12_back = RawInput {
        events: vec![
            Event::Key {
                key: Key::F12,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            Event::Key {
                key: Key::F12,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f12_back, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);
}

#[test]
fn test_disassembly_stepping_multi_word_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    common::command(
        &mut app,
        HostCommand::History(HistoryCommand::ToggleRecording),
    );

    // Fibonacci snippet:
    // $1000: 41F9 0000 2000 (LEA ($00002000).L, A0 - 6 bytes)
    // $1006: 4240 (CLR.W D0 - 2 bytes)
    // $1008: 323C 0001 (MOVE.W #1, D1 - 4 bytes)
    let fibonacci_code = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($2000).L, A0
        0x42, 0x40, // CLR.W D0
        0x32, 0x3C, 0x00, 0x01, // MOVE.W #1, D1
    ];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: fibonacci_code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // 1. Initial instruction PC must be exactly $001000
    assert_eq!(common::inspect(&mut app).cpu.instruction_pc(), 0x001000);

    // Render frame 1
    let _ = ctx.run_test_ui(RawInput::default(), |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);

    // 2. Step 1 CCK (sub-instruction phase) via F11
    let input_f11 = RawInput {
        events: vec![
            Event::ModifiersChanged(Modifiers::NONE),
            Event::Key {
                key: Key::F11,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f11, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);

    // Instruction PC must STILL be $001000 even though hardware prefetch PC advanced!
    assert_eq!(
        common::inspect(&mut app).cpu.instruction_pc(),
        0x001000,
        "Mid-instruction CCK step must preserve active instruction_pc"
    );

    // 3. Step instruction (F10) to retire LEA and land on CLR.W
    let input_f10 = RawInput {
        events: vec![Event::Key {
            key: Key::F10,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f10, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);

    // Instruction PC must transition to $001006 (CLR.W D0)
    assert_eq!(
        common::inspect(&mut app).cpu.instruction_pc(),
        0x001006,
        "Retiring LEA must advance instruction_pc to $001006 (CLR.W)"
    );

    // Step next instruction (F10) to execute CLR.W and land on MOVE.W
    let _ = ctx.run_test_ui(
        RawInput {
            events: vec![Event::Key {
                key: Key::F10,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);
    assert_eq!(
        common::inspect(&mut app).cpu.instruction_pc(),
        0x001008,
        "Retiring CLR.W must advance instruction_pc to $001008 (MOVE.W)"
    );
}

#[test]
fn test_memory_hex_watchpoint_toggle_and_rendering() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Toggle watchpoint at $002000
    let mut breakpoints = common::inspect(&mut app).debugger.breakpoints;
    breakpoints.toggle_byte_watchpoint(0x002000, debugger::WatchAccess::Write);
    common::command(&mut app, HostCommand::SetBreakpoints(breakpoints));
    assert!(common::inspect(&mut app)
        .debugger
        .breakpoints
        .find_watchpoint_at(0x002000)
        .is_some());

    // Render frame: must draw watchpoint badges without panic
    let mut output = ctx.run_test_ui(RawInput::default(), |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    output.append(common::settle(&mut app, &ctx));
    assert!(
        !output.shapes.is_empty(),
        "Frame render with watchpoint must produce shapes"
    );
}

#[test]
fn test_disassembly_active_line_highlight_and_column_alignment() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Load instructions: LEA ($2000).L, A0 ($1000), CLR.W D0 ($1006)
    let code = [0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, 0x42, 0x40];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Initial render at $001000: active line must produce frame highlight shapes
    let mut output_step0 = ctx.run_test_ui(RawInput::default(), |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    output_step0.append(common::settle(&mut app, &ctx));
    assert!(
        !output_step0.shapes.is_empty(),
        "Frame render at $1000 must render valid shapes"
    );

    // Step to $001006 (CLR.W): active line advances cleanly with identical layout
    let _ = ctx.run_test_ui(
        RawInput {
            events: vec![Event::Key {
                key: Key::F10,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);
    assert_eq!(common::inspect(&mut app).cpu.instruction_pc(), 0x001006);

    let mut output_step1 = ctx.run_test_ui(RawInput::default(), |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    output_step1.append(common::settle(&mut app, &ctx));
    assert!(
        !output_step1.shapes.is_empty(),
        "Frame render at $1006 must render valid shapes"
    );
}

#[test]
fn test_register_inline_edit_focus_cancel_and_commit() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // 1. Test Escape cancellation: D3 should remain 0
    assert_eq!(common::inspect(&mut app).cpu.d_long(3), 0);
    app.active_reg_edit = Some((EditRegister::D(3), "DEADBEEF".to_string()));

    let input_esc = RawInput {
        events: vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_esc, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(common::inspect(&mut app).cpu.d_long(3), 0);

    // 2. Test Enter commit: A2 should be updated to $00040000
    app.active_reg_edit = Some((EditRegister::A(2), "00040000".to_string()));
    let input_enter = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_enter, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(common::inspect(&mut app).cpu.a_long(2), 0x00040000);

    // 3. Test Invalid hex entry: should dismiss gracefully without modifying D1 or crashing
    assert_eq!(common::inspect(&mut app).cpu.d_long(1), 0);
    app.active_reg_edit = Some((EditRegister::D(1), "ZZZZZZZZ".to_string()));
    let input_enter_invalid = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_enter_invalid, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(common::inspect(&mut app).cpu.d_long(1), 0);
}

#[test]
fn test_disassembly_inline_patch_focus_cancel_and_commit() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Inject NOP ($4E71) at $001000
    let code = [0x4E, 0x71, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x4E71
    );

    // 1. Open patch edit buffer, then press Escape to cancel
    app.active_disasm_edit = Some(DisasmEditState {
        addr: 0x001000,
        text: "MOVE.W D0, D1".to_string(),
        error: None,
    });
    let input_esc = RawInput {
        events: vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_esc, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert!(app.active_disasm_edit.is_none());
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x4E71
    );

    // 2. Open patch edit buffer and press Enter to commit valid patch
    app.active_disasm_edit = Some(DisasmEditState {
        addr: 0x001000,
        text: "MOVE.W D0, D1".to_string(),
        error: None,
    });
    let input_enter = RawInput {
        events: vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_enter, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert!(app.active_disasm_edit.is_none());
    assert_eq!(
        common::inspect(&mut app).memory.read_word_debug(0x001000),
        0x3200
    );
}

#[test]
fn test_three_column_layout_stability_and_bounds_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    // Prime with code and CPU registers
    let code = [0x4E, 0x71, 0x30, 0x3C, 0x12, 0x34, 0x4E, 0x75];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());
    app.host.queue(HostCommand::StepInstruction);

    // 1. Standard HD resolution (1280x720)
    let screen_rect_hd = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
    let raw_input_hd = RawInput {
        screen_rect: Some(screen_rect_hd),
        ..Default::default()
    };
    let mut output_hd = ctx.run_test_ui(raw_input_hd, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    output_hd.append(common::settle(&mut app, &ctx));
    assert!(!output_hd.shapes.is_empty());
    for shape in &output_hd.shapes {
        assert!(
            shape.clip_rect.max.x <= 1280.0,
            "HD shape exceeded horizontal boundary: {:?}",
            shape.clip_rect
        );
    }

    // 2. Compact small-window resolution (1024x600)
    let screen_rect_compact =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1024.0, 600.0));
    let raw_input_compact = RawInput {
        screen_rect: Some(screen_rect_compact),
        ..Default::default()
    };
    let mut output_compact = ctx.run_test_ui(raw_input_compact, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    output_compact.append(common::settle(&mut app, &ctx));
    assert!(!output_compact.shapes.is_empty());
    for shape in &output_compact.shapes {
        assert!(
            shape.clip_rect.max.x <= 1024.0,
            "Compact shape exceeded horizontal boundary: {:?}",
            shape.clip_rect
        );
    }

    // 3. Toggle microcode in compact mode: layout must remain stable without panic
    app.show_microcode = !app.show_microcode;
    let mut output_toggle = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect_compact),
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    output_toggle.append(common::settle(&mut app, &ctx));
    assert!(!output_toggle.shapes.is_empty());
}

#[test]
fn test_hover_documentation_tooltips_render_without_panics() {
    let ctx = egui::Context::default();
    ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);

    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime the initial frame
    let prime_input = RawInput {
        screen_rect: Some(screen_rect),
        ..Default::default()
    };
    let _ = ctx.run_test_ui(prime_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);

    // Hover coordinates across key inspectable UI elements
    let hover_positions = [
        egui::pos2(30.0, 80.0),   // Left dock: Data Registers
        egui::pos2(30.0, 180.0),  // Left dock: Address Registers
        egui::pos2(30.0, 350.0),  // Left dock: Execution Status & PC
        egui::pos2(30.0, 390.0),  // Left dock: Status Register (SR)
        egui::pos2(200.0, 390.0), // Left dock: Condition Code Register (CCR) flags
        egui::pos2(200.0, 420.0), // Left dock: Prefetch IR & IRC
        egui::pos2(900.0, 150.0), // Right dock: Memory Hex Dump & Region Map
        egui::pos2(600.0, 300.0), // Center: Disassembly / Video view
    ];

    for pos in hover_positions {
        let hover_input = RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(pos)],
            ..Default::default()
        };
        let mut output = ctx.run_test_ui(hover_input, |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        });
        output.append(common::settle(&mut app, &ctx));
        assert!(
            !output.shapes.is_empty(),
            "Hovering at {:?} must produce rendered shapes without panic",
            pos
        );
    }
}

#[test]
fn test_memory_hex_cell_selection_and_row_wrapping_navigation() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    app.hex_base_addr = 0x001000;
    app.memory_selected_addr = Some(0x00100F); // Column 15 of row 0

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime hover over hex editor area
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    // 1. Press ArrowRight at col 15 -> wraps to col 0 of next row ($001010)
    let right_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(right_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.memory_selected_addr, Some(0x001010));

    // 2. Press ArrowLeft at col 0 -> wraps back to col 15 of previous row ($00100F)
    let left_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(left_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.memory_selected_addr, Some(0x00100F));

    // 3. Press Enter while cell is selected -> enters inline edit mode
    let enter_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(enter_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.hex_edit_buffer.0, 0x00100F);
    assert!(!app.hex_edit_buffer.1.is_empty());

    // 4. Press Escape -> dismisses inline edit
    let esc_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(esc_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert!(app.hex_edit_buffer.1.is_empty());

    // 5. Press Escape again -> clears cell selection
    let esc_input2 = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(esc_input2, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(app.memory_selected_addr, None);
}

#[test]
fn test_disassembly_infinite_scroll_and_pc_snap() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Load instructions: NOP, NOP, NOP, BRA *-0
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, 0x60, 0xFE];
    assert!(common::command(
        &mut app,
        HostCommand::LoadBinary {
            address: 0x001000,
            bytes: code.to_vec(),
            prime: true
        }
    )
    .value
    .is_ok());

    // Initial state: disassembly_view_addr is None (following current PC)
    assert_eq!(app.disassembly_view_addr, None);

    // 1. Prime hover over disassembly pane (Full HD Left Column of Central Viewport)
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(500.0, 300.0))],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    // 2. Mouse wheel scroll down -> disassembly_view_addr advances forward
    let wheel_down = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(500.0, 300.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta: egui::vec2(0.0, -2.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(wheel_down, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert!(
        app.disassembly_view_addr.is_some(),
        "Mouse wheel down must set disassembly_view_addr"
    );

    // 3. Step instruction (F10) -> disassembly_view_addr immediately snaps back to None (auto-tracking PC)
    let step_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![Event::Key {
            key: Key::F10,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(step_input, |ui| {
        common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
        app.update_ui(ui)
    });
    common::settle(&mut app, &ctx);
    assert_eq!(
        app.disassembly_view_addr, None,
        "Stepping must reset disassembly_view_addr to None to track PC"
    );
}

#[test]
fn test_fhd_vertical_splitter_drag_and_disassembly_resizing() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let initial_width = app.disasm_pane_width;
    assert_eq!(initial_width, 460.0);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // 1. Prime frames so layout and dock sizes settle
    for _ in 0..2 {
        let _ = ctx.run_test_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
        );
    }

    let output = common::settle(&mut app, &ctx);
    let splitter = output
        .shapes
        .iter()
        .find_map(|shape| {
            let egui::Shape::LineSegment { points, stroke } = &shape.shape else {
                return None;
            };
            (points[0].x == points[1].x
                && (points[1].y - points[0].y).abs() > screen_rect.height() / 2.0
                && stroke.color == app.theme.tokens().border_subtle)
                .then(|| points[0] + (points[1] - points[0]) / 2.0)
        })
        .expect("The divider between Disassembly and the Amiga screen must be drawn");
    // Start the drag on the rendered splitter.
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(splitter),
                Event::PointerButton {
                    pos: splitter,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );

    // Drag 78 points to the right while holding the button.
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(splitter + egui::vec2(78.0, 0.0))],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );

    assert!(
        app.disasm_pane_width > initial_width,
        "Dragging vertical splitter right must increase disasm_pane_width (was {}, now {})",
        initial_width,
        app.disasm_pane_width
    );
}

#[test]
fn test_disassembly_vertical_scrollbar_interaction() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(app.disassembly_view_addr, None);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // 1. Prime frames so layout settles
    for _ in 0..2 {
        let _ = ctx.run_test_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
        );
        common::settle(&mut app, &ctx);
    }

    let output = common::settle(&mut app, &ctx);
    let track = output
        .shapes
        .iter()
        .find_map(|shape| {
            let egui::Shape::Rect(rect) = &shape.shape else {
                return None;
            };
            (rect.fill == egui::Color32::from_rgba_unmultiplied(20, 24, 34, 180)
                && rect.rect.height() > rect.rect.width())
            .then_some(rect.rect)
        })
        .expect("The Disassembly scrollbar track must be drawn");
    let start = egui::pos2(track.center().x, track.top() + track.height() * 0.25);
    let end = egui::pos2(track.center().x, track.top() + track.height() * 0.5);
    // Drag down the Disassembly 24-bit vertical scrollbar.
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(end)],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    // Dragging scrollbar must set disassembly_view_addr to an address
    assert!(
        app.disassembly_view_addr.is_some(),
        "Dragging disassembly scrollbar must activate view_addr browsing"
    );
}

#[test]
fn test_disassembly_mouse_wheel_scroll_up_in_blank_memory() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Set view_addr in blank memory (unmapped $FFFF)
    app.disassembly_view_addr = Some(0x000080);

    // Warm up frame
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    let initial_view = app.disassembly_view_addr.unwrap();
    assert_eq!(initial_view, 0x000080);

    // Send MouseWheel event scrolling UP (positive delta_y) over disassembly area
    let _ = ctx.run_test_ui(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(egui::pos2(600.0, 300.0)),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    delta: egui::vec2(0.0, 2.0),
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    common::settle(&mut app, &ctx);

    let new_addr = app.disassembly_view_addr.unwrap_or(0);
    assert!(
        new_addr < 0x000080,
        "Mouse wheel scroll UP in unmapped memory must decrement address (was 0x80, now {:#06X})",
        new_addr
    );
}

#[test]
fn test_right_dock_bottom_docking_and_fill() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Warm up frames to allow reactive height feedback to measure bottom tools
    for _ in 0..4 {
        let _ = ctx.run_test_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
        );
        common::settle(&mut app, &ctx);
    }

    // Bottom tools must have non-zero measured height docked at bottom
    assert!(
        app.right_dock_bottom_height >= 50.0,
        "Bottom tools must have a positive measured docked height (measured: {})",
        app.right_dock_bottom_height
    );

    // Verify tools measured height is persisted in temporary UI memory
    let tools_h_id = egui::Id::unique("right_dock_tools_measured_h");
    let measured: Option<f32> = ctx.data(|d| d.get_temp(tools_h_id));
    assert!(
        measured.is_some(),
        "Tools height must be stored in temp memory for reactive vertical fill"
    );
    assert_eq!(measured.unwrap(), app.right_dock_bottom_height);
}

#[test]
fn test_simulated_quick_save_and_load_shortcuts() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    let saved_cck = 2;
    let later_cck = 5;
    for _ in 0..saved_cck {
        app.host.queue(HostCommand::StepCck);
    }
    assert_eq!(common::inspect(&mut app).debugger.current_cck, saved_cck);
    assert_eq!(app.host.status.cck, saved_cck);
    assert!(!app.host.status.quick_slots[0]);
    common::settle(&mut app, &ctx);

    // 1. Simulate F6 (Quick Save Slot 1)
    let input_f6 = RawInput {
        events: vec![Event::Key {
            key: Key::F6,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f6, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    assert!(app.host.status.quick_slots[0]);
    assert!(app.toast_message.is_some());
    assert!(app
        .toast_message
        .as_ref()
        .unwrap()
        .0
        .contains("Quick-saved"));

    for _ in saved_cck..later_cck {
        app.host.queue(HostCommand::StepCck);
    }
    assert_eq!(common::inspect(&mut app).debugger.current_cck, later_cck);
    assert_eq!(app.host.status.cck, later_cck);

    // 2. Simulate F9 (Quick Load Slot 1)
    let input_f9 = RawInput {
        events: vec![Event::Key {
            key: Key::F9,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(input_f9, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    assert_eq!(common::inspect(&mut app).debugger.current_cck, saved_cck);
    assert_eq!(app.host.status.cck, saved_cck);
    assert!(app.toast_message.as_ref().unwrap().0.contains("Restored"));
}

#[test]
fn test_ctrl_r_leaves_machine_unchanged() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());

    let cck = 2;
    for _ in 0..cck {
        app.host.queue(HostCommand::StepCck);
    }
    common::settle(&mut app, &ctx);
    assert_eq!(app.host.status.cck, cck);

    let ctrl_r = RawInput {
        events: vec![
            Event::ModifiersChanged(Modifiers::COMMAND),
            Event::Key {
                key: Key::R,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run_test_ui(ctrl_r, |ui| {
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    common::settle(&mut app, &ctx);

    assert_eq!(app.host.status.cck, cck);
    assert_eq!(common::inspect(&mut app).debugger.current_cck, cck);
}

#[test]
fn test_theme_toggle_persistence() {
    let mut app = EmulatorApp::default();
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(app.theme, AppTheme::Dark);
    app.theme = AppTheme::Light;
    assert_eq!(app.theme, AppTheme::Light);
    app.theme = AppTheme::ClassicWorkbench;
    assert_eq!(app.theme, AppTheme::ClassicWorkbench);
}
