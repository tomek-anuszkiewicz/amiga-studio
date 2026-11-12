//! Main Viewport: Amiga 4:3 CRT Display Viewport
//!
//! Renders the centered 4:3 aspect-locked video display canvas with retro CRT bezel
//! and optional floating debugger toggle button for Clean Screen mode.

use egui::{pos2, vec2, Color32, Rect, RichText, Stroke};

pub(crate) const SCREEN_ASPECT_WIDTH: f32 = 4.0;
pub(crate) const SCREEN_ASPECT_HEIGHT: f32 = 3.0;
const SCREEN_ASPECT_RATIO: f32 = SCREEN_ASPECT_WIDTH / SCREEN_ASPECT_HEIGHT;

/// Host texture refreshed from the machine after each bounded execution slice.
#[derive(Default)]
pub(crate) struct ScreenTexture(Option<egui::TextureHandle>);

impl std::fmt::Debug for ScreenTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScreenTexture").field(&self.id()).finish()
    }
}

impl ScreenTexture {
    pub(crate) fn update(&mut self, ctx: &egui::Context, session: &debugger::DebuggerSession) {
        let frame = &session.machine.denise.frame_builder;
        let size = [frame.width as usize, frame.height as usize];
        let pixels = frame.frame_buffer()[..size[0] * size[1]]
            .iter()
            .map(|pixel| {
                let [alpha, red, green, blue] = pixel.to_be_bytes();
                Color32::from_rgba_unmultiplied(red, green, blue, alpha)
            })
            .collect();
        let image = egui::ColorImage { size, pixels };
        if let Some(texture) = self.0.as_mut() {
            texture.set(image, egui::TextureOptions::NEAREST);
        } else {
            self.0 = Some(ctx.load_texture("Amiga display", image, egui::TextureOptions::NEAREST));
        }
    }

    pub(crate) fn id(&self) -> Option<egui::TextureId> {
        self.0.as_ref().map(egui::TextureHandle::id)
    }
}

pub(crate) fn render_amiga_screen(
    ui: &mut egui::Ui,
    texture: Option<egui::TextureId>,
    show_floating_debug_btn: bool,
) -> bool {
    let available = ui.available_size();
    if available.x < 50.0 || available.y < 50.0 {
        return false;
    }

    // Strict 4:3 aspect ratio calculation
    let target_aspect = SCREEN_ASPECT_RATIO;
    let (screen_w, screen_h) = if available.x / available.y > target_aspect {
        (available.y * target_aspect, available.y)
    } else {
        (available.x, available.x / target_aspect)
    };

    // Center viewport in available space
    let offset_x = (available.x - screen_w) * 0.5;
    let offset_y = (available.y - screen_h) * 0.5;

    let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
    painter.rect_filled(response.rect, 0.0, ui.visuals().panel_fill);
    let origin = response.rect.min + vec2(offset_x, offset_y);
    let screen_rect = Rect::from_min_size(origin, vec2(screen_w, screen_h));

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
            texture,
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
