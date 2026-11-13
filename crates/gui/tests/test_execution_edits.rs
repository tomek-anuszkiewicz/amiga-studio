#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{Event, Key, Modifiers, RawInput};
use gui::{DisasmEditState, EditRegister, EmulatorApp, ViewMode};

#[test]
fn replacing_a_partial_instruction_checks_its_breakpoint_before_execution() {
    for patch_instruction in [false, true] {
        let mut app = EmulatorApp::default();
        app.set_view_mode(ViewMode::Developer);
        app.session
            .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfc], true)
            .unwrap();
        app.session
            .load_binary(0x2000, &[0x52, 0x80, 0x60, 0xfc], false)
            .unwrap();
        app.session.step_cck();
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
        let ctx = egui::Context::default();
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
        assert!(app.active_reg_edit.is_none());
        assert!(app.active_disasm_edit.is_none());
        assert_eq!(app.session.machine.cpu.state.instruction_pc(), target);
        app.session.debugger.breakpoints.add_pc_breakpoint(target);
        let cck = app.session.machine.cck;
        app.session.toggle_run();
        app.session.run_frame_slice(50);
        assert!(!app.session.is_running);
        assert_eq!(
            app.session.machine.cck, cck,
            "editing at {target:x} must restart breakpoint checks"
        );
        assert_eq!(app.session.machine.cpu.state.d_long(0), 0);
    }
}
