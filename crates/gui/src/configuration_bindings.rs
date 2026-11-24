//! Draft input recording and application for the configuration editor.

use crate::configuration_edit::ConfigurationEdit;
use crate::host_keys::{amiga_key_code, host_key_name, record_frontend_key};
use crate::input_mapping::{GamepadCapture, GamepadInput, GamepadRecording};
use crate::keyboard_input::{with_inbox, KeyRecording, VIEW_SWITCH_KEY};
use crate::player_config::{
    controller_axis_name, controller_button_name, ConfigurationCatalog, FrontendAction,
    GamepadShortcut, JoystickMapping, JOYSTICK_CONTACTS,
};
use crate::EmulatorApp;

#[derive(Debug)]
pub enum BindingTarget {
    Frontend(usize, FrontendAction),
    Override {
        profile: usize,
        row: Option<usize>,
        destination: u8,
    },
    ControllerShortcut {
        profile: usize,
        row: Option<usize>,
    },
    JoystickKey(String, usize),
    JoystickButton(String, usize),
    JoystickAxis(String, usize),
}

#[derive(Debug)]
pub struct BindingRecording {
    pub(crate) controller: Option<GamepadRecording>,
    target: BindingTarget,
    label: String,
    error: Option<String>,
}

enum RecordedInput {
    Keyboard(String),
    Controller(usize),
}

impl BindingRecording {
    pub(crate) fn keyboard(ctx: &egui::Context, target: BindingTarget, label: String) -> Self {
        with_inbox(ctx, |inbox| inbox.start_recording());
        Self {
            controller: None,
            target,
            label,
            error: None,
        }
    }

    pub(crate) fn gamepad(
        ctx: &egui::Context,
        target: BindingTarget,
        label: String,
        identity: &str,
        input: GamepadInput,
    ) -> Self {
        Self {
            controller: Some(GamepadRecording {
                identity: identity.into(),
                input,
                started: ctx.cumulative_pass_nr(),
            }),
            target,
            label,
            error: None,
        }
    }

    fn apply(
        &self,
        catalog: &mut ConfigurationCatalog,
        input: RecordedInput,
    ) -> Result<(), String> {
        // Validate a proposed assignment before touching the draft, including
        // every named configuration that uses a shared mapping.
        let mut proposed = catalog.clone();
        match (&self.target, input) {
            (target, RecordedInput::Keyboard(key)) => {
                if key == VIEW_SWITCH_KEY {
                    return Err("F12 is reserved for switching views. Press another key.".into());
                }
                let physical_key = || {
                    amiga_key_code(&key).ok_or_else(|| {
                        format!(
                            "{} cannot be assigned here. Press a supported physical key.",
                            host_key_name(&key)
                        )
                    })
                };
                match target {
                    BindingTarget::Frontend(profile, action) => {
                        if *action == FrontendAction::Release && key == "Escape" {
                            return Err("Escape is reserved for the guest or browser. Choose another mouse release key.".into());
                        }
                        let binding = record_frontend_key(&key).ok_or_else(|| {
                            format!(
                                "{} cannot be used as an emulator shortcut. Press another key.",
                                host_key_name(&key)
                            )
                        })?;
                        proposed.configurations[*profile]
                            .controls
                            .host
                            .frontend
                            .assign(*action, binding.into());
                    }
                    BindingTarget::Override {
                        profile,
                        row,
                        destination,
                    } => {
                        let host = physical_key()?;
                        let overrides =
                            &mut proposed.configurations[*profile].controls.host.keyboard;
                        if let Some(row) = row {
                            overrides[*row].0 = host;
                        } else {
                            overrides.push((host, *destination));
                        }
                    }
                    BindingTarget::JoystickKey(id, index) => {
                        let key = physical_key()?;
                        let mapping = draft_mapping(&mut proposed, id)?;
                        if let Some(previous) = mapping
                            .keys
                            .iter()
                            .enumerate()
                            .position(|(row, &used)| row != *index && used == key)
                        {
                            return Err(format!(
                                "{} is assigned to {} in {}. Change that binding first or press another key.",
                                crate::host_keys::host_name_for_code(key), JOYSTICK_CONTACTS[previous], mapping.name
                            ));
                        }
                        mapping.keys[*index] = key;
                    }
                    _ => return Err("Press a controller input.".into()),
                }
            }
            (target, RecordedInput::Controller(input)) => match target {
                BindingTarget::ControllerShortcut { profile, row } => {
                    let device = self.controller.as_ref().ok_or("Select a controller.")?;
                    let shortcuts = &mut proposed.configurations[*profile]
                        .controls
                        .host
                        .gamepad_shortcuts;
                    if let Some(row) = row {
                        shortcuts[*row].button = input;
                    } else {
                        shortcuts.push(GamepadShortcut {
                            identity: device.identity.clone(),
                            button: input,
                            action: FrontendAction::Release,
                        });
                    }
                }
                BindingTarget::JoystickButton(id, index) => {
                    let mapping = draft_mapping(&mut proposed, id)?;
                    if let Some(previous) = mapping
                        .buttons
                        .iter()
                        .enumerate()
                        .position(|(row, &used)| row != *index && used == input)
                    {
                        return Err(format!(
                            "{} is assigned to {} in {}. Change that binding first or press another button.",
                            controller_button_name(input), JOYSTICK_CONTACTS[previous], mapping.name
                        ));
                    }
                    mapping.buttons[*index] = input;
                }
                BindingTarget::JoystickAxis(id, index) => {
                    let mapping = draft_mapping(&mut proposed, id)?;
                    if mapping.axes[1 - index] == input && mapping.axes[*index] != input {
                        return Err(format!(
                            "{} is already used for {} movement. Move another axis.",
                            controller_axis_name(input),
                            if *index == 0 {
                                "vertical"
                            } else {
                                "horizontal"
                            }
                        ));
                    }
                    mapping.axes[*index] = input;
                }
                _ => return Err("Press a key.".into()),
            },
        }

        for (before, after) in catalog.configurations.iter().zip(&proposed.configurations) {
            if let Err(error) = after.controls.validate(&proposed.mappings) {
                if before.controls.validate(&catalog.mappings).as_ref().err() != Some(&error) {
                    return Err(format!("{} — {error}", after.name));
                }
            }
        }
        *catalog = proposed;
        Ok(())
    }
}

fn draft_mapping<'a>(
    catalog: &'a mut ConfigurationCatalog,
    id: &str,
) -> Result<&'a mut JoystickMapping, String> {
    catalog
        .mappings
        .iter_mut()
        .find(|mapping| mapping.id == id)
        .ok_or_else(|| "This joystick mapping is unavailable.".into())
}

impl EmulatorApp {
    pub(crate) fn render_binding_recording(&mut self, ctx: &egui::Context) {
        if self
            .player
            .editor
            .as_ref()
            .and_then(|editor| editor.recording.as_ref())
            .is_none()
        {
            return;
        }
        let focused = self.keyboard_policy(ctx).focused;
        let sender = self
            .host
            .runtime
            .as_ref()
            .map(|runtime| runtime.input_sender());
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        let Some(recording) = &mut editor.recording else {
            return;
        };
        if !focused || editor.saving || editor.destination.is_some() {
            finish_recording(editor, ctx);
            return;
        }
        let captured = if let Some(request) = &recording.controller {
            match sender
                .as_ref()
                .and_then(|sender| sender.take_gamepad_recording(request))
            {
                Some(GamepadCapture::Input(input)) => Some(RecordedInput::Controller(input)),
                Some(GamepadCapture::Unavailable) => {
                    editor.error = Some("Controller recording ended because the selected controller is unavailable.".into());
                    finish_recording(editor, ctx);
                    return;
                }
                None => None,
            }
        } else {
            match with_inbox(ctx, |inbox| inbox.take_recording()) {
                KeyRecording::Received(key) => Some(RecordedInput::Keyboard(key)),
                KeyRecording::Cancelled => {
                    finish_recording(editor, ctx);
                    return;
                }
                KeyRecording::Idle | KeyRecording::Waiting => None,
            }
        };
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("input-binding-recording")).show(ctx, |ui| {
            ui.set_max_width((ctx.screen_rect().width() - 32.0).min(420.0));
            ui.heading(if recording.controller.is_some() {
                "Press a controller input"
            } else {
                "Press a key"
            });
            ui.label(&recording.label);
            if let Some(request) = &mut recording.controller {
                if let Some(pad) = self
                    .player
                    .gamepads
                    .iter()
                    .find(|pad| pad.identity == request.identity)
                {
                    ui.label(format!("Controller: {}", pad.name));
                }
                if matches!(request.input, GamepadInput::Axis { .. }) {
                    ui.label("Move the stick on the axis you want to use.");
                }
            }
            if let Some(error) = &recording.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            cancel = ui.button("Cancel recording").clicked();
        });
        if cancel {
            finish_recording(editor, ctx);
        } else if let Some(input) = captured {
            match recording.apply(&mut editor.catalog, input) {
                Ok(()) => finish_recording(editor, ctx),
                Err(error) => {
                    recording.error = Some(error);
                    ctx.request_repaint();
                }
            }
        }
    }
}

pub fn finish_recording(editor: &mut ConfigurationEdit, ctx: &egui::Context) {
    editor.recording = None;
    with_inbox(ctx, |inbox| inbox.stop_recording());
    ctx.request_repaint();
}
