#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use gui::host_video;
#[path = "../src/layout/main_viewport/amiga_screen.rs"]
#[allow(dead_code)]
pub mod amiga_screen;
use egui::{pos2, vec2, Color32, Context, ImageData, RawInput, Rect, TextureOptions, ViewportId};
use egui_kittest::{
    kittest::{by, Queryable},
    Harness,
};
use gui::{EmulatorApp, ScreenScale, ViewMode};

#[test]
fn conversion_respects_visible_rows_and_argb_alpha() {
    let info = host_video::FrameInfo {
        width: 3,
        height: 2,
        row_stride: 5,
        field_number: 0,
        publication: 1,
        cck: 0,
        generation: 1,
        interlaced: false,
        field_odd: false,
        complete: true,
    };
    let raw = [
        0xff123456, 0x80102030, 0x00112233, 0xff00ff00, 0xff00ff00, 0xffabcdef, 0xffffffff,
        0xff000000, 0xffff0000, 0xffff0000,
    ];
    let image = amiga_screen::ScreenTexture::convert(info, &raw);
    assert_eq!(image.size, [3, 2]);
    assert_eq!(
        image.pixels,
        [
            Color32::from_rgb(0x12, 0x34, 0x56),
            Color32::from_rgba_unmultiplied(0x10, 0x20, 0x30, 0x80),
            Color32::TRANSPARENT,
            Color32::from_rgb(0xab, 0xcd, 0xef),
            Color32::WHITE,
            Color32::BLACK,
        ]
    );
}

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
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut combined = egui::FullOutput::default();
    loop {
        combined.append(ctx.run(input.clone(), |ctx| app.update_ui(ctx)));
        if app.host.is_idle() && app.host.presented.is_some() {
            return combined;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Initial screen was not published"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
            let initial = render(&mut app, &ctx, width, dpi);
            let info = app.host.presented.unwrap();
            let mut captured = common::ScreenCapture::new([info.width, info.height]);
            captured.apply(&initial);
            ctx.set_zoom_factor(zoom);
            let source_size = vec2(info.width as f32, info.height as f32);
            for (scale, factor) in [
                (ScreenScale::Half, 0.5),
                (ScreenScale::One, 1.0),
                (ScreenScale::Two, 2.0),
                (ScreenScale::Four, 4.0),
            ] {
                app.screen_scale = scale;
                captured.apply(&render(&mut app, &ctx, width, dpi));
                let output = render(&mut app, &ctx, width, dpi);
                captured.apply(&output);
                let id = captured.id.as_ref().expect("No retained screen texture");
                assert_eq!(captured.options, TextureOptions::NEAREST);
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
                assert_eq!(app.host.status.cck, 0);
            }
            app.screen_scale = ScreenScale::Fit;
            let output = render(&mut app, &ctx, width, dpi);
            captured.apply(&output);
            let id = captured.id.as_ref().unwrap();
            assert_eq!(captured.options, TextureOptions::LINEAR);
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
        common::settle_harness(&mut harness);
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
                .click_accesskit();
            common::settle_harness(&mut harness);
            harness.get_by_label(label).click_accesskit();
            common::settle_harness(&mut harness);
            assert!(!egui::Popup::is_any_open(&harness.ctx));
            assert_eq!(harness.state().screen_scale, expected);
            assert_eq!(harness.state().host.status.cck, 0);
            assert_eq!(harness.ctx.zoom_factor(), 1.0);
        }
    }
}

#[test]
fn switching_filters_keeps_the_published_image_until_the_next_field() {
    let ctx = Context::default();
    let mut session = gui::DebuggerSession::new();
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let frame = &session.machine.denise.frame_builder;
    let info = host_video::FrameInfo {
        width: frame.width as usize,
        height: frame.height as usize,
        row_stride: frame.row_stride(),
        field_number: frame.frame_number,
        publication: 1,
        cck: session.machine.cck,
        generation: 1,
        interlaced: frame.interlaced,
        field_odd: frame.field_odd,
        complete: false,
    };
    let mut texture = amiga_screen::ScreenTexture::default();
    let mut image = Some(amiga_screen::ScreenTexture::convert(
        info,
        frame.frame_buffer(),
    ));
    let published = ctx.run(RawInput::default(), |ctx| {
        if let Some(image) = image.take() {
            texture.update(ctx, image);
        }
    });
    let (id, delta) = published
        .textures_delta
        .set
        .iter()
        .find(|(_, delta)| delta.image.width() == 720)
        .unwrap();
    let ImageData::Color(image) = &delta.image;
    assert_eq!(image.size, [720, 568]);
    let published_first_pixel = image.pixels[0];
    let field = session.machine.denise.frame_builder.frame_number;
    session.machine.step_cycles(40);
    assert_eq!(session.machine.denise.frame_builder.frame_number, field);
    let cck = session.machine.cck;
    session.machine.denise.frame_builder.frame_buffer_mut()[0] = 0xffff0000;

    for (scale, expected) in [
        (amiga_screen::ScreenScale::Two, TextureOptions::NEAREST),
        (amiga_screen::ScreenScale::Fit, TextureOptions::LINEAR),
    ] {
        let output = ctx.run(RawInput::default(), |_| texture.set_scale(scale));
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
        let ImageData::Color(image) = &delta.image;
        assert_eq!(image.pixels, [published_first_pixel]);
        assert_ne!(image.pixels[0], Color32::RED);
        assert_eq!(session.machine.cck, cck);
        assert_eq!(
            session.machine.denise.frame_builder.frame_buffer()[0],
            0xffff0000
        );
    }
}
