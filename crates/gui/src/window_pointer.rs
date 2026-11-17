//! Pointer edges from the application window; no global mouse listener.

use crate::host_input::HostInputEvent;

#[derive(Debug, Default)]
pub struct WindowPointer {
    buttons: [bool; 2],
}

impl WindowPointer {
    pub fn translate(&mut self, event: &egui::Event, enabled: bool) -> Option<HostInputEvent> {
        if !enabled {
            self.buttons = [false; 2];
            return None;
        }
        match event {
            egui::Event::MouseMoved(delta) => Some(HostInputEvent::MouseMotion {
                port: 0,
                dx: delta.x.round() as i32,
                dy: delta.y.round() as i32,
            }),
            egui::Event::PointerButton {
                button, pressed, ..
            } => {
                let index = match button {
                    egui::PointerButton::Primary => 0,
                    egui::PointerButton::Secondary => 1,
                    _ => return None,
                };
                self.buttons[index] = *pressed;
                Some(HostInputEvent::MouseButtons {
                    port: 0,
                    left: self.buttons[0],
                    right: self.buttons[1],
                    middle: false,
                })
            }
            _ => None,
        }
    }

    pub fn release(&mut self) {
        self.buttons = [false; 2];
    }
}
