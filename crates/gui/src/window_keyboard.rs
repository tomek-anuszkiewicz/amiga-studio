//! Full physical keyboard input through eframe's public external-event-loop API.

use crate::keyboard_input::{receive_focus, receive_key, receive_pointer_press, PhysicalKey};
use std::sync::OnceLock;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey as WinitKey},
    window::WindowId,
};

pub fn run_native(
    name: &str,
    options: eframe::NativeOptions,
    creator: eframe::AppCreator<'_>,
) -> eframe::Result {
    let event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;
    let context = OnceLock::new();
    let created_context = &context;
    let app = eframe::create_native(
        name,
        options,
        Box::new(move |cc| {
            let _ = created_context.set(cc.egui_ctx.clone());
            creator(cc)
        }),
        &event_loop,
    );
    event_loop.run_app(&mut KeyboardApplication {
        app,
        context: &context,
        primary_window: None,
        cursor_position: None,
        modifiers: ModifiersState::empty(),
    })?;
    Ok(())
}

struct KeyboardApplication<'a> {
    app: eframe::EframeWinitApplication<'a>,
    context: &'a OnceLock<egui::Context>,
    primary_window: Option<WindowId>,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    modifiers: ModifiersState,
}

impl ApplicationHandler<eframe::UserEvent> for KeyboardApplication<'_> {
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // Eframe creates the application's primary window first.
        if *self.primary_window.get_or_insert(id) == id {
            if let Some(ctx) = self.context.get() {
                match &event {
                    WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
                    WindowEvent::Focused(false) => self.modifiers = ModifiersState::empty(),
                    WindowEvent::CursorMoved { position, .. } => {
                        self.cursor_position = Some(*position)
                    }
                    WindowEvent::CursorLeft { .. } => self.cursor_position = None,
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        if let Some(position) = self.cursor_position {
                            let scale = ctx.pixels_per_point();
                            receive_pointer_press(
                                ctx,
                                egui::pos2(position.x as f32 / scale, position.y as f32 / scale),
                            );
                        }
                    }
                    _ => {}
                }
                if collect_window_key(ctx, &event) {
                    ctx.request_repaint();
                    return;
                }
                if artwork_paste_pressed(&event, self.modifiers) {
                    receive_artwork_paste(ctx, read_clipboard_image);
                }
            }
        }
        self.app.window_event(event_loop, id, event);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app.resumed(event_loop);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.app.new_events(event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: eframe::UserEvent) {
        self.app.user_event(event_loop, event);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        self.app.device_event(event_loop, id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.about_to_wait(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.app.suspended(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.app.exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.app.memory_warning(event_loop);
    }
}

fn artwork_paste_pressed(event: &WindowEvent, modifiers: ModifiersState) -> bool {
    let WindowEvent::KeyboardInput {
        event,
        is_synthetic: false,
        ..
    } = event
    else {
        return false;
    };
    let paste_modifier =
        modifiers.control_key() || (cfg!(target_os = "macos") && modifiers.super_key());
    paste_modifier
        && !modifiers.alt_key()
        && !modifiers.shift_key()
        && event.state == ElementState::Pressed
        && !event.repeat
        && (event.physical_key == WinitKey::Code(KeyCode::KeyV)
            || matches!(&event.logical_key, Key::Character(key) if key.eq_ignore_ascii_case("v")))
}

#[derive(Clone, Default)]
pub(crate) struct ArtworkPaste {
    pub(crate) active: bool,
    pub(crate) pending: Option<Result<image::RgbaImage, String>>,
}

pub(crate) fn with_artwork_paste<T>(
    ctx: &egui::Context,
    operation: impl FnOnce(&mut ArtworkPaste) -> T,
) -> T {
    ctx.data_mut(|data| {
        operation(data.get_temp_mut_or_default(egui::Id::new("library_artwork_paste")))
    })
}

/// Delivers a native paste shortcut to the active artwork editor.
///
/// The window supplies a clipboard reader and still forwards the key to eframe
/// so text fields can receive their usual paste event.
pub fn receive_artwork_paste(
    ctx: &egui::Context,
    read_image: impl FnOnce() -> Result<Option<image::RgbaImage>, String>,
) {
    if !with_artwork_paste(ctx, |paste| paste.active) {
        return;
    }
    let image = read_image().transpose();
    if image.is_some() {
        with_artwork_paste(ctx, |paste| paste.pending = image);
        ctx.request_repaint();
    }
}

fn read_clipboard_image() -> Result<Option<image::RgbaImage>, String> {
    let image = match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) {
        Ok(image) => image,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Could not read an image from the clipboard: {error}"
            ))
        }
    };
    let width = u32::try_from(image.width).map_err(|_| "Clipboard image is too wide")?;
    let height = u32::try_from(image.height).map_err(|_| "Clipboard image is too tall")?;
    image::RgbaImage::from_raw(width, height, image.bytes.into_owned())
        .map(Some)
        .ok_or_else(|| "The clipboard image has invalid pixel data".into())
}

fn collect_window_key(ctx: &egui::Context, event: &WindowEvent) -> bool {
    let key = match event {
        WindowEvent::KeyboardInput {
            event,
            is_synthetic,
            ..
        } => PhysicalKey {
            physical: match event.physical_key {
                winit::keyboard::PhysicalKey::Code(code) => format!("{code:?}"),
                other => format!("{other:?}"),
            },
            logical: format!("{:?}", event.logical_key),
            pressed: event.state == ElementState::Pressed,
            repeat: event.repeat,
            synthetic: *is_synthetic,
        },
        WindowEvent::Focused(focused) => {
            receive_focus(ctx, *focused, None);
            return false;
        }
        _ => return false,
    };
    // Forward GUI-owned keys to eframe so editors keep native IME and clipboard behavior.
    receive_key(ctx, key, None)
}
