#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Headless Integration & Unit Tests for Amiga 500 Developer Studio (gui)

use cpu::Cpu;
use debugger::inject_binary;
use debugger::temporal::TemporalHistory;
use debugger::Debugger;
use gui::theme::AppTheme;
use gui::ViewMode;
use physical_memory::PhysicalMemory;

#[test]
fn test_temporal_history_ring_buffer() {
    let mut history = TemporalHistory::new(4);
    assert_eq!(history.len(), 0);
    assert!(history.is_empty());

    let state = cpu::CpuState::default();

    // Push 3 entries
    history.record(10, 0x1000, 0x4E71, state.clone());
    history.record(20, 0x1002, 0x4E71, state.clone());
    history.record(30, 0x1004, 0x4E71, state.clone());

    assert_eq!(history.len(), 3);
    assert_eq!(history.total_recorded(), 3);
    assert_eq!(history.get_chronological(0).unwrap().cck, 10);
    assert_eq!(history.get_chronological(1).unwrap().cck, 20);
    assert_eq!(history.get_chronological(2).unwrap().cck, 30);

    // Push 2 more entries to trigger wraparound in buffer of capacity 4
    history.record(40, 0x1006, 0x4E71, state.clone());
    history.record(50, 0x1008, 0x4E71, state.clone());

    assert_eq!(history.len(), 4);
    assert_eq!(history.total_recorded(), 5);

    // Oldest surviving entry is now cck=20
    assert_eq!(history.get_chronological(0).unwrap().cck, 20);
    assert_eq!(history.get_chronological(1).unwrap().cck, 30);
    assert_eq!(history.get_chronological(2).unwrap().cck, 40);
    assert_eq!(history.get_chronological(3).unwrap().cck, 50);

    history.clear();
    assert_eq!(history.len(), 0);
    assert_eq!(history.total_recorded(), 0);
}

#[test]
fn test_binary_loader_and_prefetch_priming() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    // NOP (0x4E71), NOP (0x4E71), RTS (0x4E75)
    let code: [u8; 6] = [0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x75];
    let written = inject_binary(&mut cpu, &mut bus, 0x002000, &code, true);

    assert_eq!(written, 6);
    assert_eq!(bus.read_word_debug(0x002000), 0x4E71);
    assert_eq!(bus.read_word_debug(0x002002), 0x4E71);
    assert_eq!(bus.read_word_debug(0x002004), 0x4E75);

    // Prefetch verification
    assert_eq!(cpu.state.ir, 0x4E71);
    assert_eq!(cpu.state.prefetch, 0x4E71);
    assert_eq!(cpu.state.pc, 0x002004); // Next prefetch target
}

#[test]
fn test_debugger_stepping_and_trace_recording() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();
    let mut dbg = Debugger::new();

    // Load NOP at $001000
    let code = [0x4E, 0x71, 0x4E, 0x71];
    inject_binary(&mut cpu, &mut bus, 0x001000, &code, true);

    assert_eq!(dbg.trace.len(), 0);

    let clocks = dbg.step_instruction(&mut cpu, &mut bus);
    assert_eq!(clocks, 4); // NOP takes 4 CPU clocks
    assert_eq!(dbg.current_cck, 2); // 4 clocks = 2 CCK
    assert_eq!(dbg.trace.len(), 1);

    let entry = dbg.trace.get(0).unwrap();
    assert_eq!(entry.pc, 0x001000);
    assert_eq!(entry.opcode, 0x4E71);
    assert!(entry.disassembly.contains("NOP"));
}

#[test]
fn test_theme_variants() {
    let ctx = egui::Context::default();
    let dark = AppTheme::Dark;
    let light = AppTheme::Light;
    let wb = AppTheme::ClassicWorkbench;

    assert_ne!(dark, light);
    assert_ne!(dark, wb);

    // Verify applying each theme executes without panic
    dark.apply(&ctx);
    light.apply(&ctx);
    wb.apply(&ctx);
}

#[test]
fn test_view_mode_and_arbitrary_binary_loading() {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    // Verify injecting into high custom screen address $070000
    let custom_addr = 0x070000;
    let code = [0x4E, 0x71, 0x32, 0x00]; // NOP, MOVE.W D0, D1
    let written = inject_binary(&mut cpu, &mut bus, custom_addr, &code, true);

    assert_eq!(written, 4);
    assert_eq!(bus.read_word_debug(custom_addr), 0x4E71);
    assert_eq!(bus.read_word_debug(custom_addr + 2), 0x3200);
    assert_eq!(cpu.state.ir, 0x4E71);

    // Verify ViewMode toggling
    let mut mode = ViewMode::ScreenOnly;
    assert_eq!(mode, ViewMode::ScreenOnly);
    mode = ViewMode::Developer;
    assert_eq!(mode, ViewMode::Developer);
}

#[test]
fn test_disassembly_instruction_editing_and_size_invariance() {
    use debugger::assemble_instruction;

    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();

    // Injected at $001000: MOVE.W D0, D1 (0x3200, 2 bytes)
    let code = [0x32, 0x00];
    inject_binary(&mut cpu, &mut bus, 0x001000, &code, true);

    let (orig_disasm, byte_len) = debugger::disassemble(0x001000, |a| bus.read_word_debug(a));
    let orig_len = orig_disasm.word_count * 2;
    assert_eq!(orig_len, 2);
    assert_eq!(byte_len, 2);

    // 1. Valid replacement: "NOP" (2 bytes) -> matches orig_len
    let new_words_ok = assemble_instruction("NOP", 0x001000).unwrap();
    let new_len_ok = new_words_ok.len() * 2;
    assert_eq!(new_len_ok, orig_len); // Exactly matching 2 bytes!

    // Commit replacement
    for (i, w) in new_words_ok.iter().enumerate() {
        bus.write_word_debug(0x001000 + (i * 2) as u32, *w);
    }
    assert_eq!(bus.read_word_debug(0x001000), 0x4E71);

    // 2. Invalid replacement: "MOVE.L #$12345678, D0" (6 bytes) -> size mismatch!
    let new_words_err = assemble_instruction("33FC 0042 0007 0000", 0x001000).unwrap();
    let new_len_err = new_words_err.len() * 2;
    assert_eq!(new_len_err, 8);
    assert_ne!(new_len_err, orig_len); // Mismatch: 8 bytes vs 2 bytes -> rejected!
}

#[test]
fn test_left_dock_registers_and_microcode_rendering() {
    let ctx = egui::Context::default();
    let mut app = gui::EmulatorApp {
        show_microcode: true,
        ..Default::default()
    };

    // Run frame in Developer mode
    let output = ctx.run(egui::RawInput::default(), |ctx| app.update_ui(ctx));
    assert!(
        !output.shapes.is_empty(),
        "Left dock with registers and microcode must render valid shapes"
    );
}

#[test]
fn test_emulator_app_debug_derive() {
    let app = gui::EmulatorApp::default();
    let debug_str = format!("{:?}", app);
    assert!(debug_str.contains("EmulatorApp"));
    // Verify EmulatorApp stack footprint is compact due to boxed DebuggerSession
    assert!(
        std::mem::size_of::<gui::EmulatorApp>() < 1024,
        "EmulatorApp size {} exceeds compact stack threshold",
        std::mem::size_of::<gui::EmulatorApp>()
    );
}

#[test]
fn test_temporal_bar_rendering() {
    let ctx = egui::Context::default();
    let mut app = gui::EmulatorApp::default();
    let output = ctx.run(egui::RawInput::default(), |ctx| app.update_ui(ctx));
    assert!(!output.shapes.is_empty());
}
