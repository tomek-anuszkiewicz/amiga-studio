#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui::{pos2, vec2, Color32, Context, RawInput, Rect};
use gui::host_commands::HostCommand;
use gui::{EmulatorApp, ViewMode};

fn render(app: &mut EmulatorApp, context: &Context, size: [f32; 2]) -> egui::FullOutput {
    context.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(size[0], size[1]))),
            ..Default::default()
        },
        |context| app.update_ui(context),
    )
}

#[test]
fn loaded_guest_updates_texture_and_pause_preserves_it_at_all_viewport_sizes() {
    let context = Context::default();
    let mut session = Box::new(gui::DebuggerSession::new());
    let program =
        include_bytes!("../../machine_loop/tests/programs/display/live_stripes/program.bin");
    assert_eq!(
        session.load_binary(0x1000, program, true).unwrap(),
        program.len()
    );
    let mut app = EmulatorApp::with_session(session);
    app.view_mode = ViewMode::ScreenOnly;
    app.host.queue(HostCommand::Run);
    let mut captured = common::ScreenCapture::new([836, 570]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        captured.apply(&render(&mut app, &context, [1280.0, 720.0]));
        if app.host.status.field_number >= 2 && captured.id.is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Guest did not publish completed fields"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(app.host.status.field_number >= 2);
    app.host.queue(HostCommand::Pause);
    loop {
        captured.apply(&render(&mut app, &context, [1280.0, 720.0]));
        if !app.host.status.running
            && app.host.is_idle()
            && app
                .host
                .presented
                .is_some_and(|info| info.cck == app.host.status.cck)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Pause did not publish its boundary image"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let cck = app.host.status.cck;
    let mut paused_pixels = None;
    for size in [[1280.0, 720.0], [800.0, 900.0], [1920.0, 1080.0]] {
        let output = render(&mut app, &context, size);
        captured.apply(&output);
        let id = captured.id.unwrap();
        let pixels = captured.pixels.clone();
        assert!(pixels.contains(&Color32::from_rgb(0, 0, 34)));
        assert!(
            pixels.contains(&Color32::from_rgb(255, 0, 0))
                || pixels.contains(&Color32::from_rgb(0, 255, 0))
        );
        let (shape, mesh) = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == id => Some((shape, mesh)),
                _ => None,
            })
            .expect("video texture was not drawn");
        let bounds = mesh.calc_bounds();
        assert!((bounds.width() / bounds.height() - 836.0 / 570.0).abs() < 0.001);
        assert!(Rect::from_min_size(pos2(0.0, 0.0), vec2(size[0], size[1])).contains_rect(bounds));
        assert!(bounds.center().distance(shape.clip_rect.center()) < 1.0);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.uv == pos2(0.0, 0.0)));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.uv == pos2(1.0, 1.0)));
        assert_eq!(app.host.status.cck, cck);
        if let Some(previous) = &paused_pixels {
            assert_eq!(&pixels, previous);
        }
        paused_pixels = Some(pixels);
    }
    app.host.queue(HostCommand::Run);
    let mut changed = false;
    while std::time::Instant::now() < deadline {
        captured.apply(&render(&mut app, &context, [1280.0, 720.0]));
        if Some(&captured.pixels) != paused_pixels.as_ref() {
            changed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(changed, "resuming did not update the video texture");
    assert!(app.host.status.cck > cck);
}
