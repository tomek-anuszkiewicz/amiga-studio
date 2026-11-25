#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::TestContext as _;
use config::{A500Config, VideoStandard};
use egui::{Context, RawInput};
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::HostCommand;
use gui::{DebuggerSession, EmulatorApp, ViewMode};
use std::time::{Duration, Instant};

fn app(standard: VideoStandard) -> EmulatorApp {
    let mut session = Box::new(DebuggerSession::from_config(A500Config::standard_1mb(
        standard,
    )));
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.view_mode = ViewMode::ScreenOnly;
    app
}

#[test]
fn debugger_run_keeps_video_updates_and_pause_control_while_cpu_status_is_stopped() {
    let mut session = Box::new(DebuggerSession::new());
    session
        .load_binary(0x1000, &[0x4e, 0x72, 0x27, 0x00], true)
        .unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.view_mode = ViewMode::Developer;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .renderer(common::TextureUploads)
        .build_ui_state(
            |ui, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness.state_mut().host.queue(HostCommand::Run);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut prior_cck = 0;
    let mut prior_field = 0;
    while prior_field < 3 {
        harness.step();
        assert!(Instant::now() < deadline, "STOP did not keep video active");
        let Some(info) = harness.state().host.presented else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        if info.field_number > prior_field {
            assert!(harness.state().host.status.running);
            assert!(harness.state().host.status.stopped);
            assert!(info.cck > prior_cck);
            assert_eq!(info.field_number, prior_field + 1);
            harness.get_by_label(" \u{2022} [RUNNING] ");
            harness.get_by_label("\u{23f8} Pause");
            let size = [info.width, info.height];
            let uploads = common::TextureUploads::take();
            let (texture_id, _) = uploads
                .iter()
                .find(|(_, update)| update.image.size() == size)
                .expect("completed field was not uploaded while CPU was stopped");
            assert!(
                harness.output().shapes.iter().any(|shape| matches!(
                    &shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == *texture_id
                )),
                "uploaded field was not drawn"
            );
            prior_cck = info.cck;
            prior_field = info.field_number;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    harness.get_by_label("\u{23f8} Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    assert!(!harness.state().host.status.running);
    harness.get_by_label("\u{25b6} Run");
    harness.get_by_label(" \u{2022} [STOPPED] ");
}

#[test]
fn frame_shortcut_and_toolbar_controls_are_operable() {
    let mut app = app(VideoStandard::Pal);
    app.view_mode = ViewMode::Developer;
    let ctx = Context::default();
    common::settle(&mut app, &ctx);
    // Publish the idle debugger keyboard policy before the window events.
    let _ = ctx.run_test_ui(RawInput::default(), |ui| app.update_ui(ui));
    gui::keyboard_input::receive_key(&ctx, common::keyboard::key("ShiftLeft", true), None);
    for pressed in [true, false] {
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            common::keyboard::key("F11", pressed),
            None,
        ));
    }
    gui::keyboard_input::receive_key(&ctx, common::keyboard::key("ShiftLeft", false), None);
    common::wait(&mut app, &ctx, |app| {
        !app.host.status.running && app.host.status.field_number == 1
    });
    assert_eq!(app.host.status.field_number, 1);
    assert!(!app.host.status.running);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .build_ui_state(
            |ui, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness.get_by_label("Faster").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.speed, 2.0);
    harness.get_by_label("Slower").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.speed, 1.0);
    harness.get_by_label("1x").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.speed, 1.0);
    harness.get_by_label("Step Frame").click_accesskit();
    common::wait_harness(&mut harness, |app| {
        !app.host.status.running && app.host.status.field_number == 2
    });
    assert_eq!(harness.state().host.status.field_number, 2);
    assert!(!harness.state().host.status.running);
}
