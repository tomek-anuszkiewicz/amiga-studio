//! Full physical keyboard input through eframe's public external-event-loop API.

use crate::keyboard_input::{PhysicalKey, WindowKey, WINDOW_KEYS_ID, WINDOW_KEY_GENERATION_ID};
use std::sync::OnceLock;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
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
    })?;
    Ok(())
}

struct KeyboardApplication<'a> {
    app: eframe::EframeWinitApplication<'a>,
    context: &'a OnceLock<egui::Context>,
    primary_window: Option<WindowId>,
}

impl ApplicationHandler<eframe::UserEvent> for KeyboardApplication<'_> {
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // Eframe creates the application's primary window first.
        if *self.primary_window.get_or_insert(id) == id {
            if let Some(ctx) = self.context.get() {
                collect_window_key(ctx, &event);
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

fn collect_window_key(ctx: &egui::Context, event: &WindowEvent) {
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
        WindowEvent::Focused(false) => PhysicalKey {
            physical: "__focus_lost__".into(),
            logical: String::new(),
            pressed: false,
            repeat: false,
            synthetic: false,
        },
        _ => return,
    };
    receive_key(ctx, key);
}

/// Queue a physical key supplied by the native window backend.
pub fn receive_key(ctx: &egui::Context, key: PhysicalKey) {
    ctx.data_mut(|data| {
        let generation = data
            .get_temp::<u64>(egui::Id::new(WINDOW_KEY_GENERATION_ID))
            .unwrap_or(0);
        let events = data.get_temp_mut_or_default::<Vec<WindowKey>>(egui::Id::new(WINDOW_KEYS_ID));
        if events.len() >= crate::host_input::INPUT_QUEUE_CAPACITY {
            events.clear();
            events.push((
                "__overflow__".into(),
                String::new(),
                false,
                false,
                false,
                generation,
            ));
        }
        events.push((
            key.physical,
            key.logical,
            key.pressed,
            key.repeat,
            key.synthetic,
            generation,
        ));
    });
}
