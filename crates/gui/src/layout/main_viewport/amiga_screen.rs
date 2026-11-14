//! Main Viewport: complete Amiga framebuffer display.
//!
//! Renders the centered video display with uniform scaling and a retro CRT bezel
//! and optional floating debugger toggle button for Clean Screen mode.

use egui::{load::SizedTexture, pos2, vec2, Color32, Rect, RichText, Stroke};
use serde::{Deserialize, Serialize};

/// Amiga image scale, independent of interface zoom and host DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScreenScale {
    #[default]
    Fit,
    Half,
    One,
    Two,
    Four,
}

impl ScreenScale {
    pub(crate) const ALL: [Self; 5] = [Self::Fit, Self::Half, Self::One, Self::Two, Self::Four];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Fit => "Fit (smooth)",
            Self::Half => "0.5x",
            Self::One => "1x",
            Self::Two => "2x",
            Self::Four => "4x",
        }
    }

    fn texture_options(self) -> egui::TextureOptions {
        if self == Self::Fit {
            egui::TextureOptions::LINEAR
        } else {
            egui::TextureOptions::NEAREST
        }
    }

    fn image_size(
        self,
        source: egui::Vec2,
        available: egui::Vec2,
        pixels_per_point: f32,
    ) -> egui::Vec2 {
        let factor = match self {
            Self::Fit => return source * (available.x / source.x).min(available.y / source.y),
            Self::Half => 0.5,
            Self::One => 1.0,
            Self::Two => 2.0,
            Self::Four => 4.0,
        };
        source * (factor / pixels_per_point)
    }
}

/// Host texture refreshed after a completed field or paused debugger step.
#[derive(Default)]
pub(crate) struct ScreenTexture {
    texture: Option<egui::TextureHandle>,
    options: egui::TextureOptions,
    first_pixel: Color32,
}

impl std::fmt::Debug for ScreenTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScreenTexture").field(&self.image()).finish()
    }
}

impl ScreenTexture {
    pub(crate) fn set_scale(&mut self, scale: ScreenScale) {
        let options = scale.texture_options();
        if self.options != options {
            self.options = options;
            if let Some(texture) = &mut self.texture {
                // egui 0.31 carries sampler options in image updates. Re-send one
                // unchanged texel so a filter switch cannot expose an unfinished field.
                texture.set_partial(
                    [0, 0],
                    egui::ColorImage::new([1, 1], self.first_pixel),
                    options,
                );
            }
        }
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context, session: &debugger::DebuggerSession) {
        let frame = &session.machine.denise.frame_builder;
        let size = [frame.width as usize, frame.height as usize];
        let pixels = frame
            .frame_buffer()
            .chunks_exact(frame.row_stride())
            .flat_map(|row| &row[..frame.width as usize])
            .map(|pixel| {
                let [alpha, red, green, blue] = pixel.to_be_bytes();
                Color32::from_rgba_unmultiplied(red, green, blue, alpha)
            })
            .collect();
        let image = egui::ColorImage { size, pixels };
        self.first_pixel = image.pixels.first().copied().unwrap_or(Color32::BLACK);
        if let Some(texture) = self.texture.as_mut() {
            texture.set(image, self.options);
        } else {
            self.texture = Some(ctx.load_texture("Amiga display", image, self.options));
        }
    }

    pub(crate) fn image(&self) -> Option<SizedTexture> {
        self.texture.as_ref().map(SizedTexture::from_handle)
    }
}

pub(crate) fn render_amiga_screen(
    ui: &mut egui::Ui,
    texture: Option<SizedTexture>,
    scale: ScreenScale,
    show_floating_debug_btn: bool,
) -> bool {
    let available = ui.available_size();
    if available.x < 50.0 || available.y < 50.0 {
        return false;
    }

    let pixels_per_point = ui.ctx().pixels_per_point();
    let image_size = texture.map_or(available, |image| {
        scale.image_size(image.size, available, pixels_per_point)
    });

    let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
    let painter = painter.with_clip_rect(response.rect.intersect(ui.clip_rect()));
    painter.rect_filled(response.rect, 0.0, ui.visuals().panel_fill);
    let mut origin = response.rect.center() - image_size * 0.5;
    if scale != ScreenScale::Fit {
        origin = (origin * pixels_per_point).round() / pixels_per_point;
    }
    let screen_rect = Rect::from_min_size(origin, image_size);

    // Outer CRT Bezel
    let bezel_rect = screen_rect.expand(6.0);
    painter.rect_filled(bezel_rect, 10.0, Color32::from_rgb(15, 16, 20));
    painter.rect_stroke(
        bezel_rect,
        10.0,
        Stroke::new(2.0_f32, Color32::from_rgb(45, 48, 60)),
        egui::StrokeKind::Outside,
    );

    // Inner Phosphor Screen
    painter.rect_filled(screen_rect, 6.0, Color32::from_rgb(8, 10, 14));
    painter.rect_stroke(
        screen_rect,
        6.0,
        Stroke::new(1.0_f32, Color32::from_rgb(25, 30, 42)),
        egui::StrokeKind::Inside,
    );

    if let Some(texture) = texture {
        painter.image(
            texture.id,
            screen_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    let mut debug_clicked = false;
    if show_floating_debug_btn {
        let btn_rect = Rect::from_min_size(
            pos2(screen_rect.max.x - 145.0, screen_rect.min.y + 12.0),
            vec2(135.0, 26.0),
        );
        let resp = ui.put(
            btn_rect,
            egui::Button::new(
                RichText::new("🛠 Debugger (F12)")
                    .monospace()
                    .size(11.0)
                    .color(Color32::from_rgb(0, 240, 255)),
            )
            .fill(Color32::from_rgba_unmultiplied(20, 24, 32, 210))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0, 180, 220))),
        );
        if resp.clicked() {
            debug_clicked = true;
        }
    }

    debug_clicked
}
