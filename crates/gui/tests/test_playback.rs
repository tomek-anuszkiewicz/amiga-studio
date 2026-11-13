#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use egui::{Context, Event, Key, Modifiers, RawInput};
use egui_kittest::{kittest::Queryable, Harness};
use gui::{DebuggerSession, EmulatorApp, ViewMode};

fn app(standard: VideoStandard) -> EmulatorApp {
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;
    app.session = Box::new(DebuggerSession::from_config(A500Config::standard_1mb(
        standard,
    )));
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    app
}

fn render(app: &mut EmulatorApp, ctx: &Context, time: f64) -> egui::FullOutput {
    ctx.run(
        RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    )
}

#[test]
fn debugger_run_keeps_video_updates_and_pause_control_while_cpu_status_is_stopped() {
    let mut app = app(VideoStandard::Pal);
    app.view_mode = ViewMode::Developer;
    app.session
        .load_binary(0x1000, &[0x4e, 0x72, 0x27, 0x00], true)
        .unwrap();
    app.session.toggle_run();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    let mut prior_cck = harness.state().session.machine.cck;
    let mut prior_field = harness
        .state()
        .session
        .machine
        .denise
        .frame_builder
        .frame_number;
    for _ in 0..3 {
        harness.step();
        let session = &harness.state().session;
        assert!(session.is_running);
        assert!(session.machine.cpu.state.is_stopped());
        assert!(session.machine.cck > prior_cck);
        assert_eq!(
            session.machine.denise.frame_builder.frame_number,
            prior_field + 1
        );
        harness.get_by_label(" ● [RUNNING] ");
        harness.get_by_label("⏸ Pause (F5)");
        let frame = &session.machine.denise.frame_builder;
        let size = [frame.width as usize, frame.height as usize];
        let (texture_id, _) = harness
            .output()
            .textures_delta
            .set
            .iter()
            .find(|(_, delta)| delta.image.size() == size)
            .expect("completed field was not uploaded while CPU was stopped");
        assert!(
            harness.output().shapes.iter().any(|shape| matches!(
                &shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == *texture_id
            )),
            "uploaded field was not drawn"
        );
        prior_cck = session.machine.cck;
        prior_field = frame.frame_number;
    }
    harness.get_by_label("⏸ Pause (F5)").click();
    harness.run_steps(2);
    assert!(!harness.state().session.is_running);
    harness.get_by_label("▶ Run (F5)");
    harness.get_by_label(" ● [STOPPED] ");
}

#[test]
fn run_uses_host_time_and_measures_fields_independently_of_gui_repaints() {
    for (standard, updates_per_second) in [(VideoStandard::Pal, 120), (VideoStandard::Ntsc, 240)] {
        let mut app = app(standard);
        let ctx = Context::default();
        app.session.toggle_run();
        render(&mut app, &ctx, 0.0);
        let first_cck = app.session.machine.cck;
        let first_field = app.session.machine.denise.frame_builder.frame_number;
        assert_eq!(first_field, 1);

        // Mouse/UI repaints with no elapsed host time must not advance emulation.
        for _ in 0..3 {
            let output = render(&mut app, &ctx, 0.0);
            assert!(output.textures_delta.set.is_empty());
            assert_eq!(app.session.machine.cck, first_cck);
        }
        let mut prior_field = first_field;
        for tick in 1..=updates_per_second {
            render(
                &mut app,
                &ctx,
                f64::from(tick) / f64::from(updates_per_second),
            );
            let field = app.session.machine.denise.frame_builder.frame_number;
            assert!(
                field == prior_field || field == prior_field + 1,
                "a presented field was skipped"
            );
            prior_field = field;
        }
        let clock = f64::from(standard.color_clock_hz());
        let elapsed_cck = app.session.machine.cck - first_cck;
        // Whole-field presentation quantizes progress by less than one field.
        assert!((elapsed_cck as f64 - clock).abs() < first_cck as f64);
        let expected_fps = match standard {
            VideoStandard::Pal => 50.0,
            VideoStandard::Ntsc => 60.0,
        };
        assert!(
            (app.playback.fps() - expected_fps).abs() <= 1.0,
            "{:?}: {} FPS",
            standard,
            app.playback.fps()
        );

        app.session.toggle_run();
        let paused_cck = app.session.machine.cck;
        render(&mut app, &ctx, 30.0);
        assert_eq!(app.playback.fps(), 0.0);
        app.session.toggle_run();
        render(&mut app, &ctx, 30.0);
        assert!(app.session.machine.cck - paused_cck <= first_cck);
    }
}

#[test]
fn speed_controls_change_emulated_time_without_skipping_fields() {
    for speed in [0.25, 0.5, 2.0, 4.0] {
        let mut app = app(VideoStandard::Pal);
        while app.playback.speed() > speed {
            app.playback.slower();
        }
        while app.playback.speed() < speed {
            app.playback.faster();
        }
        let ctx = Context::default();
        app.session.toggle_run();
        render(&mut app, &ctx, 0.0);
        let first_cck = app.session.machine.cck;
        for tick in 1..=100 {
            render(&mut app, &ctx, f64::from(tick) / 1000.0);
        }
        let elapsed_cck = app.session.machine.cck - first_cck;
        let expected = f64::from(VideoStandard::Pal.color_clock_hz()) * speed * 0.1;
        assert!(
            (elapsed_cck as f64 - expected).abs() < first_cck as f64,
            "speed {speed}"
        );
    }
}

#[test]
fn accelerated_fps_counts_emulated_fields_in_both_views() {
    for view in [ViewMode::ScreenOnly, ViewMode::Developer] {
        let mut app = app(VideoStandard::Pal);
        app.view_mode = view;
        app.playback.faster();
        let ctx = Context::default();
        app.session.toggle_run();
        render(&mut app, &ctx, 0.0);
        let mut uploads = 0;
        let mut measured_fields = 0;
        for tick in 1..=50 {
            measured_fields = app.session.machine.denise.frame_builder.frame_number;
            let output = render(&mut app, &ctx, f64::from(tick) / 50.0);
            let frame = &app.session.machine.denise.frame_builder;
            uploads += output
                .textures_delta
                .set
                .iter()
                .filter(|(_, delta)| {
                    delta.image.size() == [frame.width as usize, frame.height as usize]
                })
                .count();
        }
        assert_eq!(uploads, 50);
        // Host throughput varies; the displayed sample must equal actual completed fields.
        assert_eq!(app.playback.fps(), measured_fields as f64, "{view:?}");
    }
}

#[test]
fn repeated_layout_passes_do_not_skip_a_completed_field_when_running_late() {
    let mut app = app(VideoStandard::Pal);
    let ctx = Context::default();
    app.session.toggle_run();
    render(&mut app, &ctx, 0.0);
    let initial_field = app.session.machine.denise.frame_builder.frame_number;
    let output = ctx.run(
        RawInput {
            time: Some(1.0),
            ..Default::default()
        },
        |ctx| {
            app.update_ui(ctx);
            if ctx.output(|output| output.num_completed_passes) == 0 {
                ctx.request_discard("Exercise repeated layout before presentation");
            }
        },
    );
    assert_eq!(output.platform_output.num_completed_passes, 2);
    assert_eq!(
        app.session.machine.denise.frame_builder.frame_number,
        initial_field + 1
    );
}

#[test]
fn frame_shortcut_and_toolbar_controls_are_operable() {
    let mut app = app(VideoStandard::Pal);
    app.view_mode = ViewMode::Developer;
    let ctx = Context::default();
    let _ = ctx.run(
        RawInput {
            modifiers: Modifiers::SHIFT,
            events: vec![Event::Key {
                key: Key::F11,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            }],
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    );
    assert_eq!(app.session.machine.denise.frame_builder.frame_number, 1);
    assert!(!app.session.is_running);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.get_by_label("Faster").click();
    harness.run();
    assert_eq!(harness.state().playback.speed(), 2.0);
    harness.get_by_label("Slower").click();
    harness.run();
    assert_eq!(harness.state().playback.speed(), 1.0);
    harness.get_by_label("Slower").click();
    harness.run();
    harness.get_by_label("1x").click();
    harness.run();
    assert_eq!(harness.state().playback.speed(), 1.0);
    harness.get_by_label("Step Frame (Shift+F11)").click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .session
            .machine
            .denise
            .frame_builder
            .frame_number,
        2
    );
    assert!(!harness.state().session.is_running);
}
