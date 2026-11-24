#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui::{Event, Key, Modifiers};
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::{HostCommand, HostObservation, HostValue};
use gui::{EmulatorApp, ViewMode};

fn inspection_app() -> EmulatorApp {
    let mut session = Box::new(gui::DebuggerSession::new());
    // ADDQ.L #1,D0; MOVE.W D0,$3000; BRA.S $1000.
    session
        .load_binary(
            0x1000,
            &[0x52, 0x80, 0x33, 0xc0, 0, 0, 0x30, 0, 0x60, 0xf6],
            true,
        )
        .unwrap();
    session.machine.cpu.state.set_d_long(0, 0x112233);
    session.temporal.set_recording(true);
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::Developer);
    app.hex_base_addr = 0x3000;
    app.disassembly_view_addr = Some(0x3000);
    app
}

fn texts(output: &egui::FullOutput) -> Vec<String> {
    fn visit(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => {
                let label = text.galley.text();
                if !matches!(label, "▶ Run" | "⏸ Pause" | "⏹ Stop")
                    && !label.contains(" ● [")
                    && !label.starts_with("FPS:")
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
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
            inspection_app(),
        );
    common::settle_harness(&mut harness);
    let before = texts(harness.output());
    assert!(before.iter().any(|s| s == "$00112233"));
    assert!(before.iter().any(|s| s.starts_with("00003000:")));
    harness.get_by_label("▶ Run").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.running);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let (d0, word) = live_execution(harness.state_mut(), 0, Some(0x3000));
        if d0 != 0x112233 && word.unwrap() != 0 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Running loop did not change D0 and $3000: {d0:#x}, {word:?}"
        );
        common::settle_harness(&mut harness);
    }
    assert_same_text(texts(harness.output()), before.clone());
    assert!(harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
    assert!(harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .temporal
        .is_empty());

    // Raw keyboard events must not bypass disabled inspection controls.
    harness.key_press_modifiers(Modifiers::ALT, Key::T);
    harness.key_press(Key::F10);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .temporal
        .is_recording());
    assert!(harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
    assert_same_text(texts(harness.output()), before.clone());

    harness.get_by_label("⏸ Pause").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    assert!(!harness.state().host.status.running);
    assert_ne!(texts(harness.output()), before);
    let cck = harness.state().host.status.cck;
    harness.get_by_label(&format!("CCK: {cck}"));
    for (key, modifiers) in [
        (Key::F10, Modifiers::NONE),
        (Key::F11, Modifiers::NONE),
        (Key::F11, Modifiers::SHIFT),
    ] {
        let previous_cck = harness.state().host.status.cck;
        harness.key_press_modifiers(modifiers, key);
        common::wait_harness(&mut harness, |app| {
            !app.host.status.running && app.host.status.cck > previous_cck
        });
        let cck = harness.state().host.status.cck;
        assert!(cck > previous_cck);
        harness.get_by_label(&format!("CCK: {cck}"));
    }
    assert!(!harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
    assert!(!harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .temporal
        .is_empty());
}

fn live_execution(
    app: &mut gui::EmulatorApp,
    data_register: usize,
    word_address: Option<u32>,
) -> (u32, Option<u16>) {
    let reply = common::idle_command(
        app,
        HostCommand::Observe(HostObservation::Execution {
            data_register,
            word_address,
        }),
    );
    assert!(reply.status.running);
    let Ok(HostValue::Execution {
        data_register,
        memory_word,
    }) = reply.value
    else {
        panic!("Missing live execution observation: {:?}", reply.value);
    };
    (data_register, memory_word)
}

#[test]
fn breakpoint_refreshes_the_first_stopped_frame_and_reset_load_invalidate_data() {
    let mut app = inspection_app();
    let saved = common::snapshot_json(&mut app);
    let mut breakpoints = common::inspect(&mut app).debugger.breakpoints;
    breakpoints.add_conditional_breakpoint(
        0x1000,
        debugger::BreakpointCondition::eq(debugger::ConditionRegister::D(0), 0x112234),
    );
    common::command(&mut app, HostCommand::SetBreakpoints(breakpoints));
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
            |ctx| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
        )
    };
    render(&mut app, vec![]);
    common::settle(&mut app, &ctx);
    let mut output = render(
        &mut app,
        vec![Event::Key {
            key: Key::F5,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    output.append(common::wait(&mut app, &ctx, |app| !app.host.status.running));
    assert!(!app.host.status.running);
    assert_eq!(app.host.data.as_ref().unwrap().cpu.instruction_pc(), 0x1000);
    assert_eq!(app.host.data.as_ref().unwrap().cpu.d_long(0), 0x112234);
    assert!(texts(&output).contains(&"$00112234".to_string()));
    assert!(app
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
    common::command(&mut app, HostCommand::Reset);
    let mut output = render(&mut app, vec![]);
    output.append(common::settle(&mut app, &ctx));
    assert!(texts(&output).contains(&"CCK: 0".to_string()));
    common::command(
        &mut app,
        HostCommand::RestorePlayer {
            json: saved,
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    );
    let mut output = render(&mut app, vec![]);
    output.append(common::settle(&mut app, &ctx));
    assert!(texts(&output).contains(&"$00112233".to_string()));

    // Loading through the GUI invalidates retained data even if Run starts before
    // another paused repaint. The ROM executes a tight loop at its reset vector.
    let mut rom = vec![0; 256 * 1024];
    rom[..10].copy_from_slice(&[0, 7, 0, 0, 0, 0xfc, 0, 8, 0x60, 0xfe]);
    let path = std::env::temp_dir().join(format!("amiga-frozen-rom-{}.rom", std::process::id()));
    std::fs::write(&path, &rom).unwrap();
    app.load_rom_file(&path);
    std::fs::remove_file(path).unwrap();
    app.host.queue(HostCommand::Run);
    let mut output = render(&mut app, vec![]);
    output.append(common::settle(&mut app, &ctx));
    output.append(common::wait(&mut app, &ctx, |app| {
        app.host.status.running && app.host.status.cck > 0
    }));
    assert!(app.host.status.running);
    assert!(app.host.status.cck > 0);
    assert!(texts(&output).contains(&"CCK: 0".to_string()));
    assert!(!texts(&output).contains(&"$00112233".to_string()));
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    assert_eq!(common::read_cpu(&mut app).instruction_pc(), 0xfc0008);
    assert_eq!(common::read_memory(&mut app, 0xfc0000, 10), &rom[..10]);
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
        let _ = ctx.run(input.clone(), |ctx| {
            common::keyboard::inject_taps(ctx, app.host.status.generation);
            app.update_ui(ctx)
        });
    }
    common::settle(&mut app, &ctx);
    let before = ctx.run(input.clone(), |ctx| {
        common::keyboard::inject_taps(ctx, app.host.status.generation);
        app.update_ui(ctx)
    });
    app.host.queue(HostCommand::Run);
    let output = ctx.run(input.clone(), |ctx| {
        app.update_ui(ctx);
        if ctx.output(|out| out.num_completed_passes) == 0 {
            ctx.request_discard("repeat frozen layout");
        }
    });
    assert_eq!(output.platform_output.num_completed_passes, 2);
    common::wait(&mut app, &ctx, |app| app.host.status.field_number >= 1);
    assert!(app.host.status.field_number >= 1);
    assert_same_text(texts(&output), texts(&before));

    let mut game = inspection_app();
    game.set_view_mode(ViewMode::ScreenOnly);
    game.host.queue(HostCommand::Run);
    let ctx = egui::Context::default();
    let output = ctx.run(input, |ctx| game.update_ui(ctx));
    let labels = texts(&output);
    assert!(!labels
        .iter()
        .any(|s| s == "CPU Registers" || s.starts_with("CCK:")));
    assert!(
        game.host.data.is_none(),
        "Game must not request inspector snapshots"
    );
    common::command(&mut game, HostCommand::Pause);
    common::wait(&mut game, &ctx, |app| !app.host.status.running);
    let game_data = common::inspect(&mut game);
    assert!(game_data.debugger.trace.entries().is_empty());
    assert!(game_data.previous_cpu.is_none());
    game.host.queue(HostCommand::Run);
    game.set_view_mode(ViewMode::Developer);
    let mut output = ctx.run(egui::RawInput::default(), |ctx| game.update_ui(ctx));
    output.append(common::wait(&mut game, &ctx, |app| {
        !app.host.status.running
    }));
    assert!(game
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
    assert!(!game.host.status.running);
    assert!(texts(&output).contains(&format!("CCK: {}", game.host.status.cck)));
}

#[test]
fn run_keeps_an_open_editor_inactive_across_keyboard_input_and_resize() {
    let mut app = inspection_app();
    app.active_reg_edit = Some((gui::EditRegister::D(7), "DEADBEEF".to_string()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1920.0, 1080.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness.key_press(Key::F5);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(!harness.state().host.status.running);
    assert_eq!(common::read_cpu(harness.state_mut()).d_long(7), 0);
    harness.get_by_label("▶ Run").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.state().host.status.running);
    harness.key_press(Key::Enter);
    harness.set_size(egui::vec2(1280.0, 720.0));
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert_eq!(live_execution(harness.state_mut(), 7, None), (0, None));
    assert_eq!(
        harness.state().active_reg_edit.as_ref().unwrap().1,
        "DEADBEEF"
    );
    assert_eq!(harness.state().hex_base_addr, 0x3000);
    assert_eq!(harness.state().disassembly_view_addr, Some(0x3000));
    harness.get_by_label("$00112233");
    assert!(harness
        .state()
        .host
        .data
        .as_ref()
        .unwrap()
        .debugger
        .trace
        .entries()
        .is_empty());
}

#[test]
fn binary_load_during_run_retains_inspectors_until_the_owner_stops() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1920.0, 1080.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
            inspection_app(),
        );
    common::settle_harness(&mut harness);
    harness.get_by_label("\u{25b6} Run").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.running);
    let generation = harness.state().host.status.generation;
    harness.state_mut().pending_binary = Some(gui::BinarySource::Bytes {
        name: "patch.bin".into(),
        data: vec![0x4e, 0x71],
    });
    harness.state_mut().load_binary_modal_open = true;
    harness.state_mut().load_target_addr_str = "004000".into();
    harness.state_mut().load_auto_prime = false;
    common::settle_harness(&mut harness);
    harness.get_by_label("Load into Memory").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.generation != generation);
    assert!(harness.state().host.status.running);
    assert!(
        harness.state().host.data.is_some(),
        "Running load discarded retained inspection"
    );
    assert!(!harness.state().load_binary_modal_open);
    harness.get_by_label("$00112233");
    harness.get_by_label("\u{23f8} Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    let data = harness.state().host.data.as_ref().unwrap();
    assert_eq!(data.memory.read_word_debug(0x4000), 0x4e71);
    assert_ne!(data.cpu.d_long(0), 0x112233);
}
