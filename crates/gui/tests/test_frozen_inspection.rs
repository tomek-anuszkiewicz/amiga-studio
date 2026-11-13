#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{Event, Key, Modifiers};
use egui_kittest::{kittest::Queryable, Harness};
use gui::{EmulatorApp, ViewMode};

fn inspection_app() -> EmulatorApp {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::Developer);
    // ADDQ.L #1,D0; MOVE.W D0,$3000; BRA.S $1000.
    app.session
        .load_binary(
            0x1000,
            &[0x52, 0x80, 0x33, 0xc0, 0, 0, 0x30, 0, 0x60, 0xf6],
            true,
        )
        .unwrap();
    app.session.machine.cpu.state.set_d_long(0, 0x112233);
    app.hex_base_addr = 0x3000;
    app.disassembly_view_addr = Some(0x3000);
    app.session.temporal.set_recording(true);
    app
}

fn texts(output: &egui::FullOutput) -> Vec<String> {
    fn visit(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => {
                let label = text.galley.text();
                if !label.contains("(F5)") && !label.contains(" ● [") && !label.starts_with("FPS:")
                {
                    texts.push(label.to_owned());
                }
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| visit(s, texts)),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for shape in &output.shapes {
        visit(&shape.shape, &mut texts);
    }
    texts.sort();
    texts
}

fn assert_same_text(actual: Vec<String>, expected: Vec<String>) {
    use std::collections::BTreeMap;
    let counts = |values: Vec<String>| {
        let mut map = BTreeMap::new();
        for value in values {
            *map.entry(value).or_insert(0) += 1;
        }
        map
    };
    let actual = counts(actual);
    let expected = counts(expected);
    let changed: Vec<_> = actual
        .keys()
        .chain(expected.keys())
        .filter(|key| actual.get(*key) != expected.get(*key))
        .map(|key| (key, actual.get(key), expected.get(key)))
        .collect();
    assert!(changed.is_empty(), "changed inspector text: {changed:?}");
}

#[test]
fn run_retains_inspector_text_and_rows_until_stop_then_steps_refresh() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1920.0, 1080.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| app.update_ui(ctx),
            inspection_app(),
        );
    harness.run();
    let before = texts(harness.output());
    assert!(before.iter().any(|s| s == "$00112233"));
    assert!(before.iter().any(|s| s.starts_with("00003000:")));
    harness.get_by_label("▶ Run (F5)").click();
    harness.run_steps(4);
    assert!(harness.state().session.is_running);
    assert_ne!(
        harness.state().session.machine.cpu.state.d_long(0),
        0x112233
    );
    assert_ne!(harness.state().session.bus().read_word_debug(0x3000), 0);
    assert_same_text(texts(harness.output()), before.clone());
    assert!(harness.state().session.debugger.trace.entries().is_empty());
    assert!(harness.state().session.temporal.is_empty());

    // Raw keyboard events must not bypass disabled inspection controls.
    harness.press_key_modifiers(Modifiers::ALT, Key::T);
    harness.press_key(Key::F10);
    harness.run_steps(2);
    assert!(harness.state().session.temporal.is_recording());
    assert!(harness.state().session.debugger.trace.entries().is_empty());
    assert_same_text(texts(harness.output()), before.clone());

    harness.get_by_label("⏸ Pause (F5)").click();
    harness.run_steps(2);
    assert!(!harness.state().session.is_running);
    assert_ne!(texts(harness.output()), before);
    let cck = harness.state().session.machine.cck;
    harness.get_by_label(&format!("CCK: {cck}"));
    for (key, modifiers) in [
        (Key::F10, Modifiers::NONE),
        (Key::F11, Modifiers::NONE),
        (Key::F11, Modifiers::SHIFT),
    ] {
        let previous_cck = harness.state().session.machine.cck;
        harness.press_key_modifiers(modifiers, key);
        harness.run();
        let cck = harness.state().session.machine.cck;
        assert!(cck > previous_cck);
        harness.get_by_label(&format!("CCK: {cck}"));
    }
    assert!(!harness.state().session.debugger.trace.entries().is_empty());
    assert!(!harness.state().session.temporal.is_empty());
}

#[test]
fn breakpoint_refreshes_the_first_stopped_frame_and_reset_load_invalidate_data() {
    let mut app = inspection_app();
    let saved = app.session.save_state();
    app.session.debugger.breakpoints.add_conditional_breakpoint(
        0x1000,
        debugger::BreakpointCondition::eq(debugger::ConditionRegister::D(0), 0x112234),
    );
    let ctx = egui::Context::default();
    let render = |app: &mut EmulatorApp, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.update_ui(ctx),
        )
    };
    render(&mut app, vec![]);
    let output = render(
        &mut app,
        vec![Event::Key {
            key: Key::F5,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!app.session.is_running);
    assert_eq!(app.session.machine.cpu.state.instruction_pc(), 0x1000);
    assert_eq!(app.session.machine.cpu.state.d_long(0), 0x112234);
    assert!(texts(&output).contains(&"$00112234".to_string()));
    assert!(app.session.debugger.trace.entries().is_empty());
    app.session.reset();
    let output = render(&mut app, vec![]);
    assert!(texts(&output).contains(&"CCK: 0".to_string()));
    app.session.load_state(&saved);
    let output = render(&mut app, vec![]);
    assert!(texts(&output).contains(&"$00112233".to_string()));

    // Loading through the GUI invalidates retained data even if Run starts before
    // another paused repaint. The ROM executes a tight loop at its reset vector.
    let mut rom = vec![0; 256 * 1024];
    rom[..10].copy_from_slice(&[0, 7, 0, 0, 0, 0xfc, 0, 8, 0x60, 0xfe]);
    app.load_rom_data("loop.rom", &rom);
    app.session.toggle_run();
    let output = render(&mut app, vec![]);
    assert!(app.session.is_running);
    assert!(app.session.machine.cck > 0);
    assert!(texts(&output).contains(&"CCK: 0".to_string()));
    assert!(!texts(&output).contains(&"$00112233".to_string()));
}

#[test]
fn extra_layout_pass_and_game_only_startup_do_not_inspect_live_values() {
    let mut app = inspection_app();
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1920.0, 1080.0),
        )),
        time: Some(0.0),
        ..Default::default()
    };
    for _ in 0..3 {
        let _ = ctx.run(input.clone(), |ctx| app.update_ui(ctx));
    }
    let before = ctx.run(input.clone(), |ctx| app.update_ui(ctx));
    app.session.toggle_run();
    let output = ctx.run(input.clone(), |ctx| {
        app.update_ui(ctx);
        if ctx.output(|out| out.num_completed_passes) == 0 {
            ctx.request_discard("repeat frozen layout");
        }
    });
    assert_eq!(output.platform_output.num_completed_passes, 2);
    assert_eq!(app.session.machine.denise.frame_builder.frame_number, 1);
    assert_same_text(texts(&output), texts(&before));

    let mut game = inspection_app();
    game.set_view_mode(ViewMode::ScreenOnly);
    game.session.toggle_run();
    let ctx = egui::Context::default();
    let output = ctx.run(input, |ctx| game.update_ui(ctx));
    let labels = texts(&output);
    assert!(!labels
        .iter()
        .any(|s| s == "CPU Registers" || s.starts_with("CCK:")));
    assert!(game.session.debugger.trace.entries().is_empty());
    assert!(game.session.prev_cpu_state.is_none());
    game.set_view_mode(ViewMode::Developer);
    let output = ctx.run(egui::RawInput::default(), |ctx| game.update_ui(ctx));
    assert!(!game.session.is_running);
    assert!(texts(&output).contains(&format!("CCK: {}", game.session.machine.cck)));
}

#[test]
fn run_keeps_an_open_editor_inactive_across_keyboard_input_and_resize() {
    let mut app = inspection_app();
    app.active_reg_edit = Some((gui::EditRegister::D(7), "DEADBEEF".to_string()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1920.0, 1080.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run();
    harness.press_key(Key::F5);
    harness.run_steps(2);
    assert!(harness.state().session.is_running);
    harness.press_key(Key::Enter);
    harness.set_size(egui::vec2(1280.0, 720.0));
    harness.run_steps(2);
    assert_eq!(harness.state().session.machine.cpu.state.d_long(7), 0);
    assert_eq!(
        harness.state().active_reg_edit.as_ref().unwrap().1,
        "DEADBEEF"
    );
    assert_eq!(harness.state().hex_base_addr, 0x3000);
    assert_eq!(harness.state().disassembly_view_addr, Some(0x3000));
    harness.get_by_label("$00112233");
    assert!(harness.state().session.debugger.trace.entries().is_empty());
}
