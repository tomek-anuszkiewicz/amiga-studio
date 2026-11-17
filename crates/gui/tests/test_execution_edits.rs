#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui::{Event, Key, Modifiers, RawInput};
use gui::host_commands::HostCommand;
use gui::{DisasmEditState, EditRegister, EmulatorApp, ViewMode};

#[test]
fn hex_editor_commits_ram_and_rom_bytes_through_the_owner() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session
        .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
        .unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::Developer);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    for address in [0x2000, 0xfc0000] {
        app.hex_base_addr = address;
        common::settle(&mut app, &ctx);
        app.hex_edit_buffer = (address, format!("{:02X}", 0xaa));
        common::settle(&mut app, &ctx);
        let _ = ctx.run(
            RawInput {
                events: vec![Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        );
        common::settle(&mut app, &ctx);
        assert_eq!(
            common::inspect(&mut app).memory.read_byte_debug(address),
            0xaa,
            "Hex edit at {address:06x}; remaining editor: {:?}",
            app.hex_edit_buffer
        );
    }
}

#[test]
fn breakpoint_form_reaches_the_owner_and_stops_before_execution() {
    use egui_kittest::{kittest::Queryable, Harness};
    let mut session = Box::new(gui::DebuggerSession::new());
    session
        .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
        .unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::Developer);
    app.breakpoint_form.new_pc_str = "001000".into();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    harness.get_by_label("➕ Add New PC Breakpoint").click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("Add PC Breakpoint").click_accesskit();
    common::settle_harness(&mut harness);
    let data = common::inspect(harness.state_mut());
    assert!(data.debugger.breakpoints.has_pc_breakpoint(0x1000));
    let cck = data.cck;
    common::command(harness.state_mut(), HostCommand::Run);
    let ctx = harness.ctx.clone();
    common::wait(harness.state_mut(), &ctx, |app| !app.host.status.running);
    assert_eq!(harness.state().host.status.cck, cck);
    assert_eq!(
        common::inspect(harness.state_mut()).cpu.instruction_pc(),
        0x1000
    );
}

#[test]
fn instruction_editor_patches_rom_without_requiring_a_complete_firmware_image() {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::Developer);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    common::command(
        &mut app,
        HostCommand::SetRegister {
            register: EditRegister::PC,
            value: 0xfc0000,
        },
    );
    common::settle(&mut app, &ctx);
    app.disassembly_view_addr = Some(0xfc0000);
    app.active_disasm_edit = Some(DisasmEditState {
        addr: 0xfc0000,
        text: "NOP".into(),
        error: None,
    });
    let _ = ctx.run(
        RawInput {
            events: vec![Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );
    common::settle(&mut app, &ctx);
    assert!(app.active_disasm_edit.is_none());
    let state = common::inspect(&mut app);
    assert_eq!(state.memory.read_word_debug(0xfc0000), 0x4e71);
    assert_eq!(state.cpu.instruction_pc(), 0xfc0000);
}

#[test]
fn replacing_a_partial_instruction_checks_its_breakpoint_before_execution() {
    for patch_instruction in [false, true] {
        let mut session = Box::new(gui::DebuggerSession::new());
        session
            .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
            .unwrap();
        session
            .load_binary(0x2000, &[0x52, 0x80, 0x60, 0xfc], false)
            .unwrap();
        session.step_cck();
        let mut app = EmulatorApp::with_session(session);
        app.set_view_mode(ViewMode::Developer);
        let ctx = egui::Context::default();
        common::settle(&mut app, &ctx);
        let target = if patch_instruction { 0x1000 } else { 0x2000 };
        if patch_instruction {
            app.active_disasm_edit = Some(DisasmEditState {
                addr: target,
                text: "ADDQ.L #1,D0".into(),
                error: None,
            });
        } else {
            app.active_reg_edit = Some((EditRegister::PC, "002000".into()));
        }
        let _ = ctx.run(
            RawInput {
                events: vec![Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        );
        common::settle(&mut app, &ctx);
        assert!(app.active_reg_edit.is_none());
        assert!(app.active_disasm_edit.is_none());
        assert_eq!(app.host.data.as_ref().unwrap().cpu.instruction_pc(), target);
        let mut breakpoints = app.host.data.as_ref().unwrap().debugger.breakpoints.clone();
        breakpoints.add_pc_breakpoint(target);
        common::command(&mut app, HostCommand::SetBreakpoints(breakpoints));
        let cck = app.host.status.cck;
        common::command(&mut app, HostCommand::Run);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.host.status.running {
            common::settle(&mut app, &ctx);
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        common::settle(&mut app, &ctx);
        assert!(!app.host.status.running);
        assert_eq!(
            app.host.status.cck, cck,
            "editing at {target:x} must restart breakpoint checks"
        );
        assert_eq!(app.host.data.as_ref().unwrap().cpu.d_long(0), 0);
    }
}
