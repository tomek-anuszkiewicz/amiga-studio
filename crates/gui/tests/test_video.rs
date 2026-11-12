#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{pos2, vec2, Color32, Context, ImageData, RawInput, Rect, TextureId};
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

fn video(output: &egui::FullOutput) -> (TextureId, Vec<Color32>) {
    output
        .textures_delta
        .set
        .iter()
        .find_map(|(id, delta)| {
            if delta.image.size() != [912, 313] {
                return None;
            }
            match &delta.image {
                ImageData::Color(image) => Some((*id, image.pixels.clone())),
                _ => None,
            }
        })
        .expect("machine image was not uploaded")
}

#[test]
fn loaded_guest_updates_texture_and_pause_preserves_it_at_all_viewport_sizes() {
    let context = Context::default();
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;
    let program =
        include_bytes!("../../machine_loop/tests/programs/display/live_stripes/program.bin");
    assert_eq!(
        app.session.load_binary(0x1000, program, true).unwrap(),
        program.len()
    );
    app.session.toggle_run();
    for _ in 0..32 {
        render(&mut app, &context, [1280.0, 720.0]);
        if app.session.machine.denise.frame_builder.frame_number >= 2 {
            break;
        }
    }
    assert!(app.session.machine.denise.frame_builder.frame_number >= 2);
    app.session.toggle_run();
    let cck = app.session.machine.cck;
    let mut paused_pixels = None;
    for size in [[1280.0, 720.0], [800.0, 900.0], [1920.0, 1080.0]] {
        let output = render(&mut app, &context, size);
        let (id, pixels) = video(&output);
        assert!(pixels.contains(&Color32::from_rgb(0, 0, 32)));
        assert!(
            pixels.contains(&Color32::from_rgb(240, 0, 0))
                || pixels.contains(&Color32::from_rgb(0, 240, 0))
        );
        let mesh = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == id => Some(mesh),
                _ => None,
            })
            .expect("video texture was not drawn");
        let bounds = mesh.calc_bounds();
        assert!((bounds.width() / bounds.height() - 4.0 / 3.0).abs() < 0.001);
        assert!(Rect::from_min_size(pos2(0.0, 0.0), vec2(size[0], size[1])).contains_rect(bounds));
        assert_eq!(app.session.machine.cck, cck);
        if let Some(previous) = &paused_pixels {
            assert_eq!(&pixels, previous);
        }
        paused_pixels = Some(pixels);
    }
    app.session.toggle_run();
    let mut changed = false;
    for _ in 0..16 {
        let output = render(&mut app, &context, [1280.0, 720.0]);
        if Some(video(&output).1) != paused_pixels {
            changed = true;
            break;
        }
    }
    assert!(changed, "resuming did not update the video texture");
    assert!(app.session.machine.cck > cck);
}
