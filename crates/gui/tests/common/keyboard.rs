use gui::host_input::HostInputEvent;
use gui::keyboard_input::{KeyRoute, KeyboardPolicy, KeyboardRouter, PhysicalKey};

pub fn policy() -> KeyboardPolicy {
    KeyboardPolicy {
        focused: true,
        text_focus: false,
        session: true,
        pending_launch: false,
        captured: true,
        running: true,
        pause_pending: false,
        developer: false,
        can_edit: true,
        bindings: Default::default(),
    }
}
pub fn key(physical: &str, pressed: bool) -> PhysicalKey {
    PhysicalKey {
        physical: physical.into(),
        logical: physical.into(),
        pressed,
        repeat: false,
        synthetic: false,
    }
}
pub fn translate(
    router: &mut KeyboardRouter,
    physical: &str,
    pressed: bool,
    enabled: bool,
) -> Option<HostInputEvent> {
    let mut policy = policy();
    policy.captured = enabled;
    match router.route(&key(physical, pressed), &policy) {
        KeyRoute::Amiga { code, pressed } => Some(HostInputEvent::Key { code, pressed }),
        _ => None,
    }
}
pub fn button(button: egui::PointerButton, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: egui::Pos2::ZERO,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// Legacy GUI fixtures describe isolated taps; supply the window events now used by production.
pub fn inject_taps(ctx: &egui::Context, generation: u64) {
    if !ctx.output(|output| output.num_completed_passes == 0) {
        return;
    }
    let mut raw = Vec::new();
    ctx.input(|input| {
        for event in &input.events {
            let egui::Event::Key {
                key,
                physical_key,
                pressed,
                repeat: _,
                modifiers,
            } = event
            else {
                continue;
            };
            let name = physical_key.unwrap_or(*key).name();
            let physical = if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
                format!("Key{name}")
            } else if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() {
                format!("Digit{name}")
            } else {
                match name {
                    "Backtick" => "Backquote",
                    "OpenBracket" => "BracketLeft",
                    "CloseBracket" => "BracketRight",
                    "Equals" => "Equal",
                    other => other,
                }
                .to_owned()
            };
            let modifiers = [
                ("ControlLeft", modifiers.ctrl),
                ("ShiftLeft", modifiers.shift),
                ("AltLeft", modifiers.alt),
            ];
            for (modifier, down) in modifiers {
                if down {
                    raw.push((modifier.into(), String::new(), true, false, false));
                }
            }
            raw.push((physical.clone(), key.name().into(), *pressed, false, false));
            if *pressed {
                raw.push((physical, key.name().into(), false, false, false));
            }
            for (modifier, down) in modifiers {
                if down {
                    raw.push((modifier.into(), String::new(), false, false, false));
                }
            }
        }
    });
    if !raw.is_empty() {
        let raw: Vec<_> = raw
            .into_iter()
            .map(|(p, l, d, r, s)| (p, l, d, r, s, generation))
            .collect();
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("window_keyboard_events"), raw));
    }
}
