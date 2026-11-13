#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{pos2, vec2, Color32, Context, ImageData, RawInput, Rect, TextureOptions, ViewportId};
use egui_kittest::{
    kittest::{by, Queryable},
    Harness,
};
use gui::{EmulatorApp, ScreenScale, ViewMode};

fn render(app: &mut EmulatorApp, ctx: &Context, width: f32, dpi: f32) -> egui::FullOutput {
    let mut input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 900.0))),
        time: Some(0.0),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(dpi);
    ctx.run(input, |ctx| app.update_ui(ctx))
}

#[test]
fn fixed_scales_preserve_physical_pixels_and_clip_in_every_layout() {
    for (view, width) in [
        (ViewMode::ScreenOnly, 1280.0),
        (ViewMode::Developer, 1024.0),
        (ViewMode::Developer, 1280.0),
        (ViewMode::Developer, 1920.0),
    ] {
        for (dpi, zoom) in [(1.0, 1.0), (1.5, 1.25)] {
            let ctx = Context::default();
            let mut app = EmulatorApp::default();
            app.view_mode = view;
            render(&mut app, &ctx, width, dpi);
            ctx.set_zoom_factor(zoom);
            let frame = &app.session.machine.denise.frame_builder;
            let source_size = vec2(frame.width as f32, frame.height as f32);
            for (scale, factor) in [
                (ScreenScale::Half, 0.5),
                (ScreenScale::One, 1.0),
                (ScreenScale::Two, 2.0),
                (ScreenScale::Four, 4.0),
            ] {
                app.screen_scale = scale;
                render(&mut app, &ctx, width, dpi);
                let output = render(&mut app, &ctx, width, dpi);
                let (id, delta) = output
                    .textures_delta
                    .set
                    .iter()
                    .find(|(_, delta)| {
                        delta.image.size() == [source_size.x as usize, source_size.y as usize]
                    })
                    .unwrap_or_else(|| panic!("No screen upload: {view:?}, width={width}, dpi={dpi}, zoom={zoom}, scale={scale:?}"));
                assert_eq!(delta.options, TextureOptions::NEAREST);
                let (shape, bounds) = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh) if mesh.texture_id == *id => {
                            Some((shape, mesh.calc_bounds()))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("No screen mesh: {view:?}, width={width}, dpi={dpi}, zoom={zoom}, scale={scale:?}"));
                let pixels_per_point = ctx.pixels_per_point();
                assert!((pixels_per_point - dpi * zoom).abs() < 0.001);
                assert!((bounds.width() * pixels_per_point - source_size.x * factor).abs() < 0.002);
                assert!(
                    (bounds.height() * pixels_per_point - source_size.y * factor).abs() < 0.002
                );
                for value in [
                    bounds.min.x * pixels_per_point,
                    bounds.min.y * pixels_per_point,
                ] {
                    assert!((value - value.round()).abs() < 0.002);
                }
                let center_error = (bounds.center() - shape.clip_rect.center()) * pixels_per_point;
                assert!(center_error.x.abs() <= 0.501 && center_error.y.abs() <= 0.501);
                if scale == ScreenScale::Four {
                    assert!(bounds.width() > shape.clip_rect.width());
                    assert!(bounds.height() > shape.clip_rect.height());
                }
                assert_eq!(app.session.machine.cck, 0);
            }
            app.screen_scale = ScreenScale::Fit;
            let output = render(&mut app, &ctx, width, dpi);
            let (id, delta) = output
                .textures_delta
                .set
                .iter()
                .find(|(_, delta)| {
                    delta.image.size() == [source_size.x as usize, source_size.y as usize]
                })
                .unwrap();
            assert_eq!(delta.options, TextureOptions::LINEAR);
            let (shape, bounds) = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == *id => {
                        Some((shape, mesh.calc_bounds()))
                    }
                    _ => None,
                })
                .unwrap();
            assert!(shape.clip_rect.expand(0.001).contains_rect(bounds));
            assert!(
                (bounds.width() / bounds.height() - source_size.x / source_size.y).abs() < 0.001
            );
            assert!(
                (bounds.width() - shape.clip_rect.width()).abs() < 0.001
                    || (bounds.height() - shape.clip_rect.height()).abs() < 0.001
            );
        }
    }
}

#[test]
fn selector_changes_image_scale_without_changing_machine_or_interface_zoom() {
    for view in [ViewMode::ScreenOnly, ViewMode::Developer] {
        let mut app = EmulatorApp::default();
        app.view_mode = view;
        let mut harness = Harness::builder()
            .with_size(vec2(1280.0, 900.0))
            .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
        harness.run();
        for (label, expected) in [
            ("0.5x", ScreenScale::Half),
            ("2x", ScreenScale::Two),
            ("4x", ScreenScale::Four),
            ("Fit (smooth)", ScreenScale::Fit),
        ] {
            harness
                .query_all(
                    by().role(egui::accesskit::Role::ComboBox)
                        .label("Image scale"),
                )
                .next()
                .unwrap()
                .click();
            harness.run();
            harness.get_by_label(label).click();
            harness.run();
            assert!(!harness.ctx.memory(|m| m.any_popup_open()));
            assert_eq!(harness.state().screen_scale, expected);
            assert_eq!(harness.state().session.machine.cck, 0);
            assert_eq!(harness.ctx.zoom_factor(), 1.0);
        }
    }
}

#[test]
fn switching_filters_keeps_the_published_image_until_the_next_field() {
    let ctx = Context::default();
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    app.session.toggle_run();
    let published = render(&mut app, &ctx, 1280.0, 1.0);
    let (id, delta) = published
        .textures_delta
        .set
        .iter()
        .find(|(_, delta)| delta.image.width() == 836)
        .unwrap();
    let ImageData::Color(image) = &delta.image else {
        panic!("Expected color texture")
    };
    let published_first_pixel = image.pixels[0];
    let field = app.session.machine.denise.frame_builder.frame_number;
    app.session.machine.step_cycles(40);
    assert_eq!(app.session.machine.denise.frame_builder.frame_number, field);
    let cck = app.session.machine.cck;
    app.session.machine.denise.frame_builder.frame_buffer_mut()[0] = 0xffff0000;

    for (scale, expected) in [
        (ScreenScale::Two, TextureOptions::NEAREST),
        (ScreenScale::Fit, TextureOptions::LINEAR),
    ] {
        app.screen_scale = scale;
        let output = render(&mut app, &ctx, 1280.0, 1.0);
        let updates: Vec<_> = output
            .textures_delta
            .set
            .iter()
            .filter(|(texture, _)| texture == id)
            .collect();
        assert_eq!(updates.len(), 1);
        let delta = &updates[0].1;
        assert_eq!(delta.options, expected);
        assert_eq!(delta.pos, Some([0, 0]));
        assert_eq!(delta.image.size(), [1, 1]);
        let ImageData::Color(image) = &delta.image else {
            panic!("Expected color texture")
        };
        assert_eq!(image.pixels, [published_first_pixel]);
        assert_ne!(image.pixels[0], Color32::RED);
        assert_eq!(app.session.machine.cck, cck);
        assert_eq!(
            app.session.machine.denise.frame_builder.frame_buffer()[0],
            0xffff0000
        );
    }
}
