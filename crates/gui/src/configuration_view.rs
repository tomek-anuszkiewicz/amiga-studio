//! Separate configuration and reusable mapping editor.

use crate::configuration_bindings::{BindingRecording, BindingTarget};
use crate::configuration_edit::ConfigurationEdit;
use crate::host_keys::{
    amiga_destinations, amiga_key_name, frontend_binding_name, frontend_key_position,
    host_name_for_code,
};
use crate::input_mapping::{GamepadInfo, GamepadInput};
use crate::player_config::{
    controller_axis_name, controller_button_name, FrontendAction, InputSource, JoystickMapping,
    NamedConfiguration, JOYSTICK_CONTACTS, MAX_CONTROLLER_THRESHOLD, MIN_CONTROLLER_THRESHOLD,
};
use crate::EmulatorApp;
use debugger::launch::DeviceKind;

impl EmulatorApp {
    pub(crate) fn render_configuration_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurations");
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        let catalog = &mut editor.catalog;
        let available: Vec<_> = self
            .player
            .gamepads
            .iter()
            .filter(|pad| {
                self.player
                    .gamepads
                    .iter()
                    .filter(|other| pad.identity == other.identity)
                    .count()
                    == 1
            })
            .collect();
        crate::player_view::configuration_selector(ui, catalog);
        if ui.button("Create configuration").clicked() {
            let id = format!(
                "configuration-{}-{}",
                crate::host_clock::utc_now(),
                catalog.configurations.len()
            );
            let configuration = NamedConfiguration {
                id: id.clone(),
                name: "New configuration".into(),
                ..Default::default()
            };
            catalog.configurations.push(configuration);
            catalog.selected = id;
        }
        let Some(index) = catalog
            .configurations
            .iter()
            .position(|c| c.id == catalog.selected)
        else {
            return;
        };
        let configuration = &mut catalog.configurations[index];
        ui.monospace(format!("ID: {}", configuration.id));
        ui.label("Name");
        ui.text_edit_singleline(&mut configuration.name);
        ui.label("Hardware, ROM and drives apply to a later launch.");
        egui::ComboBox::from_label("Hardware")
            .selected_text(format!("{:?}", configuration.launch.preset))
            .show_ui(ui, |ui| {
                for preset in [
                    config::A500Preset::Bare512k,
                    config::A500Preset::Standard1Mb,
                    config::A500Preset::ExpandedPowerUser,
                ] {
                    ui.selectable_value(
                        &mut configuration.launch.preset,
                        preset,
                        format!("{preset:?}"),
                    );
                }
            });
        egui::ComboBox::from_label("Video")
            .selected_text(format!("{:?}", configuration.launch.video))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut configuration.launch.video,
                    config::VideoStandard::Pal,
                    "PAL",
                );
                ui.selectable_value(
                    &mut configuration.launch.video,
                    config::VideoStandard::Ntsc,
                    "NTSC",
                );
            });
        ui.label("Kickstart (256 KiB)");
        ui.text_edit_singleline(&mut configuration.launch.rom).on_hover_text("Native file path; browser display name only. Select browser ROM bytes again in manual setup.");
        ui.horizontal_wrapped(|ui| {
            for drive in 0..4 {
                ui.add_enabled(
                    drive != 0,
                    egui::Checkbox::new(
                        &mut configuration.launch.drives[drive],
                        format!("DF{drive}: connected"),
                    ),
                );
            }
        });
        ui.add(
            egui::Slider::new(
                &mut configuration.controls.host.mouse_sensitivity,
                crate::player_config::MIN_MOUSE_SENSITIVITY
                    ..=crate::player_config::MAX_MOUSE_SENSITIVITY,
            )
            .text("Mouse sensitivity"),
        );
        for port in 0..2 {
            ui.separator();
            ui.strong(format!("Amiga port {}", port + 1));
            let binding = &mut configuration.controls.ports[port];
            let previous = binding.device;
            egui::ComboBox::from_id_salt(("port_device", port))
                .selected_text(format!("{:?}", binding.device))
                .show_ui(ui, |ui| {
                    for device in [
                        DeviceKind::Disconnected,
                        DeviceKind::Mouse,
                        DeviceKind::Joystick,
                    ] {
                        ui.selectable_value(&mut binding.device, device, format!("{device:?}"));
                    }
                });
            if binding.device != previous {
                binding.source = match binding.device {
                    DeviceKind::Disconnected => InputSource::None,
                    DeviceKind::Mouse => InputSource::SystemMouse,
                    DeviceKind::Joystick => catalog
                        .mappings
                        .first()
                        .map_or(InputSource::None, |mapping| {
                            InputSource::Keyboard(mapping.id.clone())
                        }),
                };
            }
            if binding.device == DeviceKind::Joystick {
                egui::ComboBox::from_id_salt(("port_source", port))
                    .selected_text(match &binding.source {
                        InputSource::Keyboard(id) => catalog
                            .mappings
                            .iter()
                            .find(|map| &map.id == id)
                            .map_or_else(
                                || "Keyboard mapping unavailable".into(),
                                |map| format!("Keyboard: {}", map.name),
                            ),
                        InputSource::Gamepad { identity, .. } => available
                            .iter()
                            .find(|pad| &pad.identity == identity)
                            .map_or_else(
                                || format!("Unavailable controller: {identity}"),
                                |pad| format!("Controller: {}", pad.name),
                            ),
                        _ => "Choose source".into(),
                    })
                    .show_ui(ui, |ui| {
                        for mapping in &catalog.mappings {
                            ui.selectable_value(
                                &mut binding.source,
                                InputSource::Keyboard(mapping.id.clone()),
                                format!("Keyboard: {}", mapping.name),
                            );
                        }
                        for pad in &available {
                            let mapping = match &binding.source {
                                InputSource::Gamepad { mapping, .. } => mapping.clone(),
                                _ => catalog
                                    .mappings
                                    .first()
                                    .map_or(String::new(), |m| m.id.clone()),
                            };
                            ui.selectable_value(
                                &mut binding.source,
                                InputSource::Gamepad {
                                    identity: pad.identity.clone(),
                                    mapping,
                                },
                                format!("{} ({})", pad.name, pad.identity),
                            );
                        }
                    });
                if let InputSource::Gamepad { identity, mapping } = &mut binding.source {
                    if !available.iter().any(|p| &p.identity == identity) {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "Controller unavailable; select a current connection",
                        );
                    }
                    egui::ComboBox::from_id_salt(("pad_mapping", port))
                        .selected_text(
                            catalog
                                .mappings
                                .iter()
                                .find(|candidate| candidate.id == *mapping)
                                .map_or("Choose a mapping", |candidate| candidate.name.as_str()),
                        )
                        .show_ui(ui, |ui| {
                            for candidate in &catalog.mappings {
                                ui.selectable_value(mapping, candidate.id.clone(), &candidate.name);
                            }
                        });
                }
            }
        }
        render_emulator_shortcuts(ui, editor, index);
        render_controller_shortcuts(ui, editor, index, &available);
        render_key_overrides(ui, editor, index);
        render_joystick_mappings(
            ui,
            editor,
            &available,
            self.player.active_configuration.as_ref(),
        );
        if let Err(error) = editor
            .catalog
            .validate(&editor.catalog.configurations[index])
        {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}

fn render_emulator_shortcuts(ui: &mut egui::Ui, editor: &mut ConfigurationEdit, profile: usize) {
    let bindings = &mut editor.catalog.configurations[profile]
        .controls
        .host
        .frontend;
    ui.separator();
    ui.heading("Emulator shortcuts");
    ui.label(format!(
        "{}: {}",
        FrontendAction::Debug.label(),
        crate::keyboard_input::VIEW_SWITCH_KEY
    ));
    ui.label("Release mouse: middle mouse button. You can also assign a key.");
    for action in FrontendAction::EDITABLE_KEYS {
        ui.push_id(action.label(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(action.label());
                let binding = bindings.binding(action);
                if ui.button(frontend_binding_name(binding)).clicked() {
                    editor.recording = Some(BindingRecording::keyboard(
                        ui.ctx(),
                        BindingTarget::Frontend(profile, action),
                        action.label().into(),
                    ));
                }
                if action == FrontendAction::Release
                    && !binding.is_empty()
                    && ui.button("Unassign").clicked()
                {
                    bindings.release.clear();
                }
            });
        });
    }
    if frontend_key_position(&bindings.release) == Some("Escape") {
        ui.label("Escape is reserved for the guest or browser. Use middle mouse or assign another mouse release key.");
    }
}

fn render_controller_shortcuts(
    ui: &mut egui::Ui,
    editor: &mut ConfigurationEdit,
    profile: usize,
    available: &[&GamepadInfo],
) {
    let Some(first) = available.first() else {
        ui.label("No controller detected");
        ui.collapsing("Controller shortcuts", |ui| {
            ui.disable();
            for shortcut in &editor.catalog.configurations[profile]
                .controls
                .host
                .gamepad_shortcuts
            {
                ui.label(format!(
                    "Controller: {} (unavailable) · Button: {} · Action: {}",
                    shortcut.identity,
                    controller_button_name(shortcut.button),
                    shortcut.action.label()
                ));
            }
            let _ = ui.button("Add shortcut");
        });
        return;
    };

    let shortcuts = &mut editor.catalog.configurations[profile]
        .controls
        .host
        .gamepad_shortcuts;
    ui.collapsing("Controller shortcuts", |ui| {
        let mut remove = None;
        for (row, shortcut) in shortcuts.iter_mut().enumerate() {
            ui.push_id(row, |ui| {
                ui.horizontal_wrapped(|ui| {
                    controller_selector(ui, &mut shortcut.identity, available);
                    ui.label("Button");
                    if ui
                        .add_enabled(
                            available
                                .iter()
                                .any(|pad| pad.identity == shortcut.identity),
                            egui::Button::new(controller_button_name(shortcut.button)),
                        )
                        .clicked()
                    {
                        editor.recording = Some(BindingRecording::gamepad(
                            ui.ctx(),
                            BindingTarget::ControllerShortcut {
                                profile,
                                row: Some(row),
                            },
                            shortcut.action.label().into(),
                            &shortcut.identity,
                            GamepadInput::Button,
                        ));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Action");
                    egui::ComboBox::from_id_salt("shortcut_action")
                        .selected_text(shortcut.action.label())
                        .show_ui(ui, |ui| {
                            for action in FrontendAction::ALL {
                                ui.selectable_value(&mut shortcut.action, action, action.label());
                            }
                        });
                    if ui.button("Remove shortcut").clicked() {
                        remove = Some(row);
                    }
                });
            });
        }
        if let Some(row) = remove {
            shortcuts.remove(row);
        }
        ui.push_id("new_shortcut", |ui| {
            ui.horizontal_wrapped(|ui| {
                let identity = editor
                    .mapping_controller
                    .get_or_insert_with(|| first.identity.clone());
                if !available.iter().any(|pad| pad.identity == *identity) {
                    *identity = first.identity.clone();
                }
                controller_selector(ui, identity, available);
                if ui.button("Add shortcut").clicked() {
                    editor.recording = Some(BindingRecording::gamepad(
                        ui.ctx(),
                        BindingTarget::ControllerShortcut { profile, row: None },
                        "Add shortcut".into(),
                        identity,
                        GamepadInput::Button,
                    ));
                }
            });
        });
    });
}

fn controller_selector(ui: &mut egui::Ui, identity: &mut String, available: &[&GamepadInfo]) {
    ui.label("Controller");
    let name = available
        .iter()
        .find(|pad| pad.identity == *identity)
        .map_or_else(
            || format!("Unavailable: {identity}"),
            |pad| pad.name.clone(),
        );
    egui::ComboBox::from_id_salt("recording_controller")
        .width(ui.available_width().min(185.0))
        .truncate()
        .selected_text(&name)
        .show_ui(ui, |ui| {
            for pad in available {
                ui.selectable_value(identity, pad.identity.clone(), &pad.name);
            }
        })
        .response
        .on_hover_text(name);
}

fn render_key_overrides(ui: &mut egui::Ui, editor: &mut ConfigurationEdit, profile: usize) {
    let overrides = &mut editor.catalog.configurations[profile]
        .controls
        .host
        .keyboard;
    ui.collapsing("Amiga keyboard remapping", |ui| {
        let mut remove = None;
        for (row, (host, destination)) in overrides.iter_mut().enumerate() {
            ui.push_id(row, |ui| {
                ui.horizontal_wrapped(|ui| {
                    guest_key_selector(ui, destination);
                    if ui.button(host_name_for_code(*host)).clicked() {
                        editor.recording = Some(BindingRecording::keyboard(
                            ui.ctx(),
                            BindingTarget::Override {
                                profile,
                                row: Some(row),
                                destination: *destination,
                            },
                            format!("Host key for Amiga {}", amiga_key_name(*destination)),
                        ));
                    }
                    if ui.button("Remove override").clicked() {
                        remove = Some(row);
                    }
                });
            });
        }
        if let Some(row) = remove {
            overrides.remove(row);
        }
        ui.push_id("new_override", |ui| {
            ui.horizontal_wrapped(|ui| {
                guest_key_selector(ui, &mut editor.new_guest_key);
                if ui.button("Add key override").clicked() {
                    editor.recording = Some(BindingRecording::keyboard(
                        ui.ctx(),
                        BindingTarget::Override {
                            profile,
                            row: None,
                            destination: editor.new_guest_key,
                        },
                        format!(
                            "Host key for Amiga {}",
                            amiga_key_name(editor.new_guest_key)
                        ),
                    ));
                }
            });
        });
    });
}

fn guest_key_selector(ui: &mut egui::Ui, destination: &mut u8) {
    ui.label("Amiga key");
    egui::ComboBox::from_id_salt("amiga_key")
        .selected_text(amiga_key_name(*destination))
        .show_ui(ui, |ui| {
            for key in amiga_destinations() {
                ui.selectable_value(destination, key, amiga_key_name(key));
            }
        });
}

fn render_joystick_mappings(
    ui: &mut egui::Ui,
    editor: &mut ConfigurationEdit,
    available: &[&GamepadInfo],
    active: Option<&NamedConfiguration>,
) {
    ui.separator();
    ui.heading("Reusable joystick mappings");
    if ui.button("Create mapping").clicked() {
        editor.expanded_mapping = Some(editor.catalog.create_mapping());
    }
    let mut remove = None;
    for mapping in &mut editor.catalog.mappings {
        let open = editor.expanded_mapping.as_ref() == Some(&mapping.id);
        egui::CollapsingHeader::new(&mapping.name)
            .id_salt(&mapping.id)
            .default_open(open)
            .show(ui, |ui| {
                let mapping_ui = egui::UiBuilder::new().id_salt(&mapping.id);
                ui.scope_builder(mapping_ui, |ui| {
                    ui.label("Mapping name");
                    ui.text_edit_singleline(&mut mapping.name);
                    let controller = if let Some(first) = available.first() {
                        let identity = editor
                            .mapping_controller
                            .get_or_insert_with(|| first.identity.clone());
                        if !available.iter().any(|pad| pad.identity == *identity) {
                            *identity = first.identity.clone();
                        }
                        ui.horizontal_wrapped(|ui| controller_selector(ui, identity, available));
                        Some(identity.as_str())
                    } else {
                        ui.label("Connect a controller to record buttons and axes.");
                        None
                    };
                    render_joystick_bindings(ui, mapping, controller, &mut editor.recording);

                    let named = editor
                        .catalog
                        .configurations
                        .iter()
                        .filter(|configuration| configuration.controls.uses_mapping(&mapping.id));
                    let session = active
                        .filter(|configuration| configuration.controls.uses_mapping(&mapping.id));
                    let uses: Vec<_> =
                        named
                            .map(|configuration| format!("Configuration: {}", configuration.name))
                            .chain(session.map(|configuration| {
                                format!("Active session: {}", configuration.name)
                            }))
                            .collect();
                    if ui
                        .add_enabled(uses.is_empty(), egui::Button::new("Delete mapping"))
                        .on_disabled_hover_text(
                            "Reassign or remove the listed uses before deleting this mapping.",
                        )
                        .clicked()
                    {
                        remove = Some(mapping.id.clone());
                    }
                    for usage in uses {
                        ui.label(usage);
                    }
                });
            });
    }
    editor.expanded_mapping = None;
    if let Some(id) = remove {
        editor.catalog.mappings.retain(|mapping| mapping.id != id);
    }
}

fn render_joystick_bindings(
    ui: &mut egui::Ui,
    mapping: &mut JoystickMapping,
    controller: Option<&str>,
    recording: &mut Option<BindingRecording>,
) {
    for (index, label) in JOYSTICK_CONTACTS.into_iter().enumerate() {
        ui.push_id(index, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(label);
                if ui.button(host_name_for_code(mapping.keys[index])).clicked() {
                    *recording = Some(BindingRecording::keyboard(
                        ui.ctx(),
                        BindingTarget::JoystickKey(mapping.id.clone(), index),
                        format!("{} — {label}", mapping.name),
                    ));
                }
                if ui
                    .add_enabled(
                        controller.is_some(),
                        egui::Button::new(controller_button_name(mapping.buttons[index])),
                    )
                    .clicked()
                {
                    if let Some(identity) = controller {
                        *recording = Some(BindingRecording::gamepad(
                            ui.ctx(),
                            BindingTarget::JoystickButton(mapping.id.clone(), index),
                            format!("{} — {label}", mapping.name),
                            identity,
                            GamepadInput::Button,
                        ));
                    }
                }
            });
        });
    }
    for (index, label) in ["Horizontal axis", "Vertical axis"].into_iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.label(label);
            if ui
                .add_enabled(
                    controller.is_some(),
                    egui::Button::new(controller_axis_name(mapping.axes[index])),
                )
                .clicked()
            {
                if let Some(identity) = controller {
                    *recording = Some(BindingRecording::gamepad(
                        ui.ctx(),
                        BindingTarget::JoystickAxis(mapping.id.clone(), index),
                        format!("{} — {label}", mapping.name),
                        identity,
                        GamepadInput::Axis {
                            threshold: mapping.threshold,
                        },
                    ));
                }
            }
        });
    }
    ui.add(
        egui::Slider::new(
            &mut mapping.threshold,
            MIN_CONTROLLER_THRESHOLD..=MAX_CONTROLLER_THRESHOLD,
        )
        .text("Axis threshold / deadzone"),
    );
}
