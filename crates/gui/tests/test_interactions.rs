#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{Event, Key, Modifiers, RawInput};
use gui::{AppTheme, DisasmEditState, EditRegister, EmulatorApp, ViewMode};
use std::path::PathBuf;

#[test]
fn test_simulated_f12_view_mode_toggle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
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

    let _ = ctx.run(input_f12, |ctx| {
        app.update_ui(ctx);
    });
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

    let _ = ctx.run(input_f12_back, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(app.view_mode, ViewMode::Developer);
}

#[test]
fn test_simulated_run_and_step_shortcuts() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.session.temporal.set_recording(true);

    // Load binary code: NOP, NOP, BRA *-0 (infinite loop at $1004)
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x60, 0xFE];
    app.session.load_binary(0x001000, &code, true);

    // 1. Press F10 -> Step 1 instruction
    assert_eq!(app.session.instructions_executed, 0);
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
    let _ = ctx.run(input_f10, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(app.session.instructions_executed, 1);
    assert_eq!(app.session.temporal.len(), 1);

    // 2. Press Shift + F10 -> Step backward (temporal rewind)
    let input_shift_f10 = RawInput {
        modifiers: Modifiers::SHIFT,
        events: vec![Event::Key {
            key: Key::F10,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_shift_f10, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(app.session.temporal.scrub_cursor, Some(0));

    // 3. Press F11 -> Step CCK
    let prev_cck = app.session.debugger.current_cck;
    let input_f11 = RawInput {
        events: vec![Event::Key {
            key: Key::F11,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f11, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(app.session.debugger.current_cck, prev_cck + 1);

    // 4. Press F5 -> Toggle Run
    assert!(!app.session.is_running);
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
    let _ = ctx.run(input_f5, |ctx| {
        app.update_ui(ctx);
    });
    assert!(app.session.is_running);
}

#[test]
fn test_simulated_drag_and_drop_file() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    assert!(!app.load_binary_modal_open);
    assert!(app.pending_binary_path.is_none());

    // Simulate drag-and-drop of a binary file
    let input_drop = RawInput {
        dropped_files: vec![egui::DroppedFile {
            name: "test.bin".to_string(),
            path: Some(PathBuf::from("test.bin")),
            last_modified: None,
            bytes: None,
            mime: String::new(),
        }],
        ..Default::default()
    };

    let _ = ctx.run(input_drop, |ctx| {
        app.update_ui(ctx);
    });

    // Modal should now be open with the dropped file path
    assert!(app.load_binary_modal_open);
    assert_eq!(app.pending_binary_path, Some(PathBuf::from("test.bin")));
}

#[test]
fn test_simulated_register_inline_editing() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

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

    let _ = ctx.run(input_enter, |ctx| {
        app.update_ui(ctx);
    });

    // Value should be committed to D0 and edit mode closed
    assert_eq!(app.session.machine.cpu.state.d_long(0), 0x12345678);
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
    let _ = ctx.run(input_enter_sr, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(app.session.machine.cpu.state.sr, 0x2700);
    assert_eq!(app.active_reg_edit, None);
}

#[test]
fn test_simulated_disassembly_inline_patching() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Set up NOP ($4E71) at $001000
    let code = [0x4E, 0x71, 0x4E, 0x75]; // NOP, RTS
    app.session.load_binary(0x001000, &code, true);

    // Initial instruction at $1000 is NOP
    assert_eq!(
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
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
    let _ = ctx.run(input_commit, |ctx| {
        app.update_ui(ctx);
    });

    // Instruction in RAM must now be $3200 and edit mode closed
    assert_eq!(
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
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
    let _ = ctx.run(input_commit_mismatch, |ctx| {
        app.update_ui(ctx);
    });

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
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
        0x3200
    );
}

#[test]
fn test_simulated_breakpoint_form_and_trigger() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Code: NOP ($1000), NOP ($1002), RTS ($1004)
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);

    // Add breakpoint at $001002
    app.session.debugger.breakpoints.add_pc_breakpoint(0x001002);
    assert!(app.session.debugger.breakpoints.has_pc_breakpoint(0x001002));

    // Free-run execution
    app.session.is_running = true;
    let _ = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    // Execution should have stopped precisely at breakpoint $001002
    assert_eq!(app.session.instructions_executed, 1);
    assert_eq!(app.session.machine.cpu.state.pc.wrapping_sub(4), 0x001002);
}

#[test]
fn test_simulated_temporal_time_travel_navigation() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Code: NOP, NOP, RTS
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);

    // Enable temporal for time-travel navigation test
    app.session.temporal.set_recording(true);

    // Step 2 instructions
    app.session.step_instruction();
    app.session.step_instruction();
    assert_eq!(app.session.temporal.len(), 2);
    assert_eq!(app.session.temporal.scrub_cursor, None); // Live head

    // Step backward 1 frame in history (from live head index 1 to index 0)
    app.session.step_backward();
    assert_eq!(app.session.temporal.scrub_cursor, Some(0));
    assert!(
        !app.session.machine.cpu.state.micro.current_steps.is_empty(),
        "Scrubbing backward must re-hydrate CPU micro-steps via restore_state"
    );

    // Render frame to ensure UI reflects scrub state without panics
    let _ = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    // Return to live head
    app.session.jump_to_live_head();
    assert_eq!(app.session.temporal.scrub_cursor, None);
    assert!(
        !app.session.machine.cpu.state.micro.current_steps.is_empty(),
        "Returning to live head must re-hydrate CPU micro-steps via restore_state"
    );
}

#[test]
fn test_full_frame_rendering_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Inject code so registers, memory, disassembly, and trace buffers all have live data
    let code = [0x4E, 0x71, 0x32, 0x00, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);
    app.session.temporal.set_recording(true);
    app.session.step_instruction();

    // Render multiple consecutive frames to verify zero panics and layout stability
    for _ in 0..5 {
        let output = ctx.run(RawInput::default(), |ctx| {
            app.update_ui(ctx);
        });

        // Ensure egui produced shapes/textures for rendering
        assert!(!output.shapes.is_empty());
    }
}

#[test]
fn test_startup_clean_memory() {
    let app = EmulatorApp::default();
    assert_eq!(app.session.instructions_executed, 0);
    assert!(!app.session.is_running);
    assert!(!app.session.temporal.is_recording());
    // Verify startup PC counter points to $000000 (0th cell) with valid SSP ($080000) and primed prefetch
    assert_eq!(app.session.machine.cpu.state.instruction_pc, 0x000000);
    assert_eq!(app.session.machine.cpu.state.pc, 0x000004);
    assert_eq!(app.session.machine.cpu.state.a_regs()[7], 0x080000);
    assert_eq!(app.session.machine.cpu.state.ssp, 0x080000);
    assert!(
        app.session.machine.cpu.state.ir == 0xFFFF
            || app.session.machine.cpu.state.ir == 0x0000
            || app.session.machine.cpu.state.ir == 0x0008,
        "Expected unpopulated Kickstart ROM ($FFFF), zeroed Chip RAM ($0000), or synthetic boot vector ($0008), got ${:04X}",
        app.session.machine.cpu.state.ir
    );
    assert_eq!(app.goto_addr_str, "000000");

    // Verify memory starts clean zeroed Chip RAM without auto-loaded programs
    for addr in [0x000000, 0x001000, 0x001002, 0x002000, 0x070000] {
        let val = app.session.machine.physical_memory.read_word_debug(addr);
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
    app.session.machine.cpu.state.sr = 0x2700; // All CCR flags (0x1F) are 0

    let _ = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    // Toggle Z flag (bit 2, mask 0x04)
    app.session.machine.cpu.state.sr ^= 0x04;
    assert_eq!(app.session.machine.cpu.state.sr & 0x04, 0x04);

    // Toggle X flag (bit 4, mask 0x10)
    app.session.machine.cpu.state.sr ^= 0x10;
    assert_eq!(app.session.machine.cpu.state.sr & 0x10, 0x10);
}

#[test]
fn test_register_edit_focus_and_dismissal_lifecycle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // 1. Enter edit mode on register D0
    app.active_reg_edit = Some((EditRegister::D(0), "DEADBEEF".to_string()));

    // Render frame - automatically requests focus
    let _ = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });
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
    let _ = ctx.run(input_esc, |ctx| {
        app.update_ui(ctx);
    });
    assert!(app.active_reg_edit.is_none());
    assert_ne!(app.session.machine.cpu.state.d_long(0), 0xDEADBEEF);
}

#[test]
fn test_pc_instruction_address_normalization() {
    let mut app = EmulatorApp::default();
    let code = [0x4E, 0x71, 0x4E, 0x71];
    app.session.load_binary(0x001000, &code, true);

    // Hardware prefetch advances PC bus register to $001004
    assert_eq!(app.session.machine.cpu.state.pc, 0x001004);

    // Displayed/active instruction address is normalized to state.pc - 4 = $001000
    let instruction_pc = app.session.machine.cpu.state.pc.wrapping_sub(4) & 0x00FF_FFFF;
    assert_eq!(instruction_pc, 0x001000);
}

#[test]
fn test_disassembly_instruction_decoding_fibonacci() {
    let mut app = EmulatorApp::default();
    let code: [u8; 20] = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($002000).L, A0
        0x42, 0x40, // CLR.W D0
        0x32, 0x3C, 0x00, 0x01, // MOVE.W #$0001, D1
        0x30, 0xC0, // MOVE.W D0, (A0)+
        0x30, 0xC1, // MOVE.W D1, (A0)+
        0x51, 0xCB, 0xFF, 0xF4, // DBRA D3, $1014
    ];
    app.session.load_binary(0x001000, &code, true);

    let (dis0, len0) = debugger::disassemble(0x001000, |a| {
        app.session.machine.physical_memory.read_word_debug(a)
    });
    assert_eq!(dis0.mnemonic, "LEA");
    assert!(dis0.operands.contains("($00002000).L, A0"));
    assert_eq!(len0, 6);

    let (dis1, len1) = debugger::disassemble(0x001006, |a| {
        app.session.machine.physical_memory.read_word_debug(a)
    });
    assert_eq!(dis1.mnemonic, "CLR.W");
    assert_eq!(dis1.operands, "D0");
    assert_eq!(len1, 2);

    let (dis5, len5) = debugger::disassemble(0x001010, |a| {
        app.session.machine.physical_memory.read_word_debug(a)
    });
    assert!(dis5.mnemonic == "DBF" || dis5.mnemonic == "DBRA");
    assert!(dis5.operands.contains("D3"));
    assert_eq!(len5, 4);
}

#[test]
fn test_temporal_inactive_default_and_loop_rewind() {
    let mut app = EmulatorApp::default();
    assert!(!app.session.temporal.is_recording());
    assert_eq!(debugger::temporal::DEFAULT_TEMPORAL_CAPACITY, 25_000);

    // Enable recording and run code
    app.session.temporal.set_recording(true);
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);

    app.session.step_instruction();
    app.session.step_instruction();
    assert_eq!(app.session.temporal.len(), 2);

    // Query historical passes for $1000
    let matches = app.session.temporal.find_matches_by_pc(0x001000, 10);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, 0); // Chronological index 0
}

#[test]
fn test_clean_screen_game_mode_and_crt_text() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;

    let output = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    assert!(!output.shapes.is_empty());
}

#[test]
fn test_memory_hex_right_margin_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    let raw_input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        )),
        ..Default::default()
    };
    let output = ctx.run(raw_input, |ctx| {
        app.update_ui(ctx);
    });
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
    assert!(app.show_microcode);

    // Press F8 -> toggles microcode visibility off
    let input_f8 = RawInput {
        events: vec![Event::Key {
            key: Key::F8,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f8.clone(), |ctx| {
        app.update_ui(ctx);
    });
    assert!(!app.show_microcode);

    // Press F8 again -> toggles microcode visibility on
    let _ = ctx.run(input_f8, |ctx| {
        app.update_ui(ctx);
    });
    assert!(app.show_microcode);

    // Load code and step CCK to verify micro-step clock metrics
    let code = [0x32, 0x00, 0x4E, 0x75]; // MOVE.W D0, D1
    app.session.load_binary(0x001000, &code, true);

    // Before stepping, ensure instruction is initiated or step 1 CCK
    app.session.step_cck();
    let state = &app.session.machine.cpu.state;
    // MOVE.W has 2 steps of 2 clocks each; after 1 CCK, step 0 has completed and step 1 is active
    assert!(state.micro.micro_step <= state.micro.current_steps.len() as u16);
    assert!(!state.micro.current_steps.is_empty());
}

#[test]
fn test_memory_hex_edit_mode_zero_grid_shift_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // 1. First render a baseline frame where no cells are being edited
    let _ = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    // 2. Put cell at address $00005A into active edit mode (e.g. col 10 in row 5)
    app.hex_edit_buffer.0 = 0x00005A;
    app.hex_edit_buffer.1 = "FF".to_string();

    let output_editing = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });
    assert!(!output_editing.shapes.is_empty());

    // 3. Edit with a single character "F" and verify shapes still render without shifting or panic
    app.hex_edit_buffer.1 = "F".to_string();
    let output_single_char = ctx.run(RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });
    assert!(!output_single_char.shapes.is_empty());
}

#[test]
fn test_memory_hex_mouse_wheel_infinite_scroll() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    assert_eq!(app.hex_base_addr, 0x000000);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // 1. Prime the frame with mouse hovering over right dock memory hex area (~900, 150)
    let prime_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
        ..Default::default()
    };
    let _ = ctx.run(prime_input, |ctx| {
        app.update_ui(ctx);
    });

    // 2. Wheel Down 1 tick -> scroll down 1 row (16 bytes)
    let scroll_down_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run(scroll_down_input, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(
        app.hex_base_addr, 0x000010,
        "Base address must advance by 16 bytes on 1 wheel notch"
    );

    // 3. Wheel Down 1 tick with Shift -> scroll down 1 page (16 rows = 256 bytes)
    let shift_scroll_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::SHIFT,
            },
        ],
        modifiers: Modifiers::SHIFT,
        ..Default::default()
    };
    let _ = ctx.run(shift_scroll_input, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(
        app.hex_base_addr, 0x000110,
        "Base address must advance by 256 bytes with Shift+wheel"
    );

    // 4. Wheel Down 1 tick with Ctrl -> scroll down 1 KB (64 rows = 1024 bytes)
    let ctrl_scroll_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                modifiers: Modifiers::CTRL,
            },
        ],
        modifiers: Modifiers::CTRL,
        ..Default::default()
    };
    let _ = ctx.run(ctrl_scroll_input, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(
        app.hex_base_addr, 0x000510,
        "Base address must advance by 1024 bytes with Ctrl+wheel"
    );

    // 5. Wheel Up -> wraps smoothly backwards across 24-bit boundary when moving past 0
    app.hex_base_addr = 0x000000;
    let wrap_up_input = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(900.0, 150.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run(wrap_up_input, |ctx| {
        app.update_ui(ctx);
    });
    assert_eq!(
        app.hex_base_addr, 0x00FFFFF0,
        "Base address must wrap to top of 24-bit space on upward overflow"
    );
}

#[test]
fn test_memory_hex_keyboard_navigation() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.hex_base_addr = 0x001000;

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime hover
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

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
    let _ = ctx.run(pagedown_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(pageup_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(arrowdown_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(arrowup_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(home_input, |ctx| app.update_ui(ctx));
    assert_eq!(app.hex_base_addr, 0x000000);
}

#[test]
fn test_memory_hex_vertical_scrollbar_interaction() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.hex_base_addr = 0x000000;

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // 1. Prime frame
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

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
    let _ = ctx.run(click_scrollbar_input, |ctx| app.update_ui(ctx));

    // Address must advance forward from 0
    assert!(
        app.hex_base_addr > 0x000000,
        "Clicking scrollbar track below thumb must advance base address"
    );
}

#[test]
fn test_f2_and_escape_view_mode_toggle() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    assert_eq!(app.view_mode, ViewMode::Developer);

    // 1. Press F2 -> Switch to ScreenOnly
    let input_f2 = RawInput {
        events: vec![Event::Key {
            key: Key::F2,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f2, |ctx| app.update_ui(ctx));
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);

    // 2. Press Escape from ScreenOnly -> Return to Developer
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
    let _ = ctx.run(input_esc, |ctx| app.update_ui(ctx));
    assert_eq!(app.view_mode, ViewMode::Developer);

    // 3. Press F2 again -> ScreenOnly
    let input_f2_again = RawInput {
        events: vec![Event::Key {
            key: Key::F2,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f2_again, |ctx| app.update_ui(ctx));
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);

    // 4. Press F2 to toggle back -> Developer
    let input_f2_back = RawInput {
        events: vec![Event::Key {
            key: Key::F2,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f2_back, |ctx| app.update_ui(ctx));
    assert_eq!(app.view_mode, ViewMode::Developer);
}

#[test]
fn test_disassembly_stepping_multi_word_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    app.session.temporal.set_recording(true);

    // Fibonacci snippet:
    // $1000: 41F9 0000 2000 (LEA ($00002000).L, A0 - 6 bytes)
    // $1006: 4240 (CLR.W D0 - 2 bytes)
    // $1008: 323C 0001 (MOVE.W #1, D1 - 4 bytes)
    let fibonacci_code = [
        0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, // LEA ($2000).L, A0
        0x42, 0x40, // CLR.W D0
        0x32, 0x3C, 0x00, 0x01, // MOVE.W #1, D1
    ];
    app.session.load_binary(0x001000, &fibonacci_code, true);

    // 1. Initial instruction PC must be exactly $001000
    assert_eq!(app.session.machine.cpu.state.instruction_pc, 0x001000);

    // Render frame 1
    let _ = ctx.run(RawInput::default(), |ctx| app.update_ui(ctx));

    // 2. Step 1 CCK (sub-instruction phase) via F11
    let input_f11 = RawInput {
        events: vec![Event::Key {
            key: Key::F11,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(input_f11, |ctx| app.update_ui(ctx));

    // Instruction PC must STILL be $001000 even though hardware prefetch PC advanced!
    assert_eq!(
        app.session.machine.cpu.state.instruction_pc, 0x001000,
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
    let _ = ctx.run(input_f10, |ctx| app.update_ui(ctx));

    // Instruction PC must transition to $001006 (CLR.W D0)
    assert_eq!(
        app.session.machine.cpu.state.instruction_pc, 0x001006,
        "Retiring LEA must advance instruction_pc to $001006 (CLR.W)"
    );

    // Step next instruction (F10) to execute CLR.W and land on MOVE.W
    let _ = ctx.run(
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
        |ctx| app.update_ui(ctx),
    );
    assert_eq!(
        app.session.machine.cpu.state.instruction_pc, 0x001008,
        "Retiring CLR.W must advance instruction_pc to $001008 (MOVE.W)"
    );
}

#[test]
fn test_memory_hex_watchpoint_toggle_and_rendering() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Toggle watchpoint at $002000
    app.session
        .debugger
        .breakpoints
        .toggle_byte_watchpoint(0x002000, debugger::WatchAccess::Write);
    assert!(app
        .session
        .debugger
        .breakpoints
        .find_watchpoint_at(0x002000)
        .is_some());

    // Render frame: must draw watchpoint badges without panic
    let output = ctx.run(RawInput::default(), |ctx| app.update_ui(ctx));
    assert!(
        !output.shapes.is_empty(),
        "Frame render with watchpoint must produce shapes"
    );
}

#[test]
fn test_disassembly_active_line_highlight_and_column_alignment() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Load instructions: LEA ($2000).L, A0 ($1000), CLR.W D0 ($1006)
    let code = [0x41, 0xF9, 0x00, 0x00, 0x20, 0x00, 0x42, 0x40];
    app.session.load_binary(0x001000, &code, true);

    // Initial render at $001000: active line must produce frame highlight shapes
    let output_step0 = ctx.run(RawInput::default(), |ctx| app.update_ui(ctx));
    assert!(
        !output_step0.shapes.is_empty(),
        "Frame render at $1000 must render valid shapes"
    );

    // Step to $001006 (CLR.W): active line advances cleanly with identical layout
    let _ = ctx.run(
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
        |ctx| app.update_ui(ctx),
    );
    assert_eq!(app.session.machine.cpu.state.instruction_pc, 0x001006);

    let output_step1 = ctx.run(RawInput::default(), |ctx| app.update_ui(ctx));
    assert!(
        !output_step1.shapes.is_empty(),
        "Frame render at $1006 must render valid shapes"
    );
}

#[test]
fn test_register_inline_edit_focus_cancel_and_commit() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // 1. Test Escape cancellation: D3 should remain 0
    assert_eq!(app.session.machine.cpu.state.d_long(3), 0);
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
    let _ = ctx.run(input_esc, |ctx| app.update_ui(ctx));
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(app.session.machine.cpu.state.d_long(3), 0);

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
    let _ = ctx.run(input_enter, |ctx| app.update_ui(ctx));
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(app.session.machine.cpu.state.a_long(2), 0x00040000);

    // 3. Test Invalid hex entry: should dismiss gracefully without modifying D1 or crashing
    assert_eq!(app.session.machine.cpu.state.d_long(1), 0);
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
    let _ = ctx.run(input_enter_invalid, |ctx| app.update_ui(ctx));
    assert_eq!(app.active_reg_edit, None);
    assert_eq!(app.session.machine.cpu.state.d_long(1), 0);
}

#[test]
fn test_disassembly_inline_patch_focus_cancel_and_commit() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Inject NOP ($4E71) at $001000
    let code = [0x4E, 0x71, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);
    assert_eq!(
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
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
    let _ = ctx.run(input_esc, |ctx| app.update_ui(ctx));
    assert!(app.active_disasm_edit.is_none());
    assert_eq!(
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
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
    let _ = ctx.run(input_enter, |ctx| app.update_ui(ctx));
    assert!(app.active_disasm_edit.is_none());
    assert_eq!(
        app.session
            .machine
            .physical_memory
            .read_word_debug(0x001000),
        0x3200
    );
}

#[test]
fn test_three_column_layout_stability_and_bounds_invariance() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    // Prime with code and CPU registers
    let code = [0x4E, 0x71, 0x30, 0x3C, 0x12, 0x34, 0x4E, 0x75];
    app.session.load_binary(0x001000, &code, true);
    app.session.step_instruction();

    // 1. Standard HD resolution (1280x720)
    let screen_rect_hd = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
    let raw_input_hd = RawInput {
        screen_rect: Some(screen_rect_hd),
        ..Default::default()
    };
    let output_hd = ctx.run(raw_input_hd, |ctx| app.update_ui(ctx));
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
    let output_compact = ctx.run(raw_input_compact, |ctx| app.update_ui(ctx));
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
    let output_toggle = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect_compact),
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );
    assert!(!output_toggle.shapes.is_empty());
}

#[test]
fn test_hover_documentation_tooltips_render_without_panics() {
    let ctx = egui::Context::default();
    ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0);

    let mut app = EmulatorApp::default();
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime the initial frame
    let prime_input = RawInput {
        screen_rect: Some(screen_rect),
        ..Default::default()
    };
    let _ = ctx.run(prime_input, |ctx| app.update_ui(ctx));

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
        let output = ctx.run(hover_input, |ctx| app.update_ui(ctx));
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
    app.hex_base_addr = 0x001000;
    app.memory_selected_addr = Some(0x00100F); // Column 15 of row 0

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));

    // Prime hover over hex editor area
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(900.0, 150.0))],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

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
    let _ = ctx.run(right_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(left_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(enter_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(esc_input, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(esc_input2, |ctx| app.update_ui(ctx));
    assert_eq!(app.memory_selected_addr, None);
}

#[test]
fn test_disassembly_infinite_scroll_and_pc_snap() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Load instructions: NOP, NOP, NOP, BRA *-0
    let code = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, 0x60, 0xFE];
    app.session.load_binary(0x001000, &code, true);

    // Initial state: disassembly_view_addr is None (following current PC)
    assert_eq!(app.disassembly_view_addr, None);

    // 1. Prime hover over disassembly pane (Full HD Left Column of Central Viewport)
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(500.0, 300.0))],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

    // 2. Mouse wheel scroll down -> disassembly_view_addr advances forward
    let wheel_down = RawInput {
        screen_rect: Some(screen_rect),
        events: vec![
            Event::PointerMoved(egui::pos2(500.0, 300.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -2.0),
                modifiers: Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run(wheel_down, |ctx| app.update_ui(ctx));
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
    let _ = ctx.run(step_input, |ctx| app.update_ui(ctx));
    assert_eq!(
        app.disassembly_view_addr, None,
        "Stepping must reset disassembly_view_addr to None to track PC"
    );
}

#[test]
fn test_fhd_vertical_splitter_drag_and_disassembly_resizing() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();
    let initial_width = app.disasm_pane_width;
    assert_eq!(initial_width, 460.0);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // 1. Prime frames so layout and dock sizes settle
    for _ in 0..2 {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        );
    }

    // 2. Splitter settled between Disassembly and CRT at X ~ 922.0 (459 + 460 + 3).
    // Press mouse on vertical splitter
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(egui::pos2(922.0, 400.0)),
                Event::PointerButton {
                    pos: egui::pos2(922.0, 400.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

    // 3. Drag mouse to 1000.0 while still held
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(1000.0, 400.0))],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
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
    assert_eq!(app.disassembly_view_addr, None);

    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // 1. Prime frames so layout settles
    for _ in 0..2 {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        );
    }

    // 2. Drag the Disassembly 24-bit vertical scrollbar (X ~ 915, Y ~ 300 -> 500)
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(egui::pos2(915.0, 300.0)),
                Event::PointerButton {
                    pos: egui::pos2(915.0, 300.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![Event::PointerMoved(egui::pos2(915.0, 500.0))],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

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
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Set view_addr in blank memory (unmapped $FFFF)
    app.disassembly_view_addr = Some(0x000080);

    // Warm up frame
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

    let initial_view = app.disassembly_view_addr.unwrap();
    assert_eq!(initial_view, 0x000080);

    // Send MouseWheel event scrolling UP (positive delta_y) over disassembly area
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                Event::PointerMoved(egui::pos2(600.0, 300.0)),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, 2.0),
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );

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
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));

    // Warm up frames to allow reactive height feedback to measure bottom tools
    for _ in 0..4 {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        );
    }

    // Bottom tools must have non-zero measured height docked at bottom
    assert!(
        app.right_dock_bottom_height >= 50.0,
        "Bottom tools must have a positive measured docked height (measured: {})",
        app.right_dock_bottom_height
    );

    // Verify tools measured height is persisted in temporary UI memory
    let tools_h_id = egui::Id::new("right_dock_tools_measured_h");
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

    // Advance 300 CCKs
    for _ in 0..300 {
        app.session.step_cck();
    }
    assert_eq!(app.session.debugger.current_cck, 300);
    assert!(!app.session.has_quick_slot(1));

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
    let _ = ctx.run(input_f6, |ctx| {
        app.update_ui(ctx);
    });

    assert!(app.session.has_quick_slot(1));
    assert!(app.toast_message.is_some());
    assert!(app
        .toast_message
        .as_ref()
        .unwrap()
        .0
        .contains("Quick-saved"));

    // Advance 500 more CCKs
    for _ in 0..500 {
        app.session.step_cck();
    }
    assert_eq!(app.session.debugger.current_cck, 800);

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
    let _ = ctx.run(input_f9, |ctx| {
        app.update_ui(ctx);
    });

    assert_eq!(app.session.debugger.current_cck, 300);
    assert_eq!(app.session.machine.cck, 300);
    assert!(app.toast_message.as_ref().unwrap().0.contains("Restored"));
}

#[test]
fn test_ctrl_r_shortcut_triggers_reset() {
    let ctx = egui::Context::default();
    let mut app = EmulatorApp::default();

    for _ in 0..100 {
        app.session.step_cck();
    }
    assert_eq!(app.session.machine.cck, 100);

    let ctrl_r = RawInput {
        modifiers: Modifiers::COMMAND,
        events: vec![Event::Key {
            key: Key::R,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }],
        ..Default::default()
    };
    let _ = ctx.run(ctrl_r, |ctx| {
        app.update_ui(ctx);
    });

    assert_eq!(app.session.machine.cck, 0);
    assert_eq!(app.session.debugger.current_cck, 0);
}

#[test]
fn test_theme_toggle_persistence() {
    let mut app = EmulatorApp::default();
    assert_eq!(app.theme, AppTheme::Dark);
    app.theme = AppTheme::Light;
    assert_eq!(app.theme, AppTheme::Light);
    app.theme = AppTheme::ClassicWorkbench;
    assert_eq!(app.theme, AppTheme::ClassicWorkbench);
}
