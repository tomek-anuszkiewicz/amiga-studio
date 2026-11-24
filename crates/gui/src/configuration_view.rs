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
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.render_library_setting(ui);
            if self
                .player
                .editor
                .as_ref()
                .is_some_and(|editor| editor.library_root != self.player.library_root)
            {
                ui.label("Save to open the selected library and edit its profiles.");
                return;
            }
            if !self.player.library_available() {
                ui.label("Choose an accessible library directory to create or edit profiles.");
                return;
            }
            let can_delete = self.configuration_deletion_allowed();
            if ui.add_enabled(can_delete, egui::Button::new("Delete configuration"))
                .on_hover_text_at_pointer("Review which games use the selected profile and choose a replacement if needed. Save any profile edits first.")
                .clicked()
            {
                self.begin_configuration_deletion();
            }
        }
        let active = self
            .player
            .active_configuration
            .as_ref()
            .filter(|_| self.player.is_active_library());
        let active_mappings = &self.player.active_mappings;
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
            let timestamp = crate::host_clock::utc_now();
            let mut serial = catalog.configurations.len();
            let id = loop {
                let id = format!("configuration-{timestamp}-{serial}");
                if catalog.configurations.iter().all(|cfg| cfg.id != id)
                    && self
                        .player
                        .active_configuration
                        .as_ref()
                        .is_none_or(|cfg| cfg.id != id)
                {
                    break id;
                }
                serial += 1;
            };
            let default_mapping = JoystickMapping::default();
            if catalog.mapping(&default_mapping.id).is_none() {
                catalog.mappings.push(default_mapping);
            }
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
        hardware_settings(ui, &mut configuration.launch);
        ui.label("Kickstart (256 KiB)");
        #[cfg(not(target_arch = "wasm32"))]
        let rom_help = "Use the path to a 256 KiB ROM inside the library, with / between folders.";
        #[cfg(target_arch = "wasm32")]
        let rom_help =
            "The browser stores the display name. Select the ROM file again in Manual setup.";
        ui.text_edit_singleline(&mut configuration.launch.rom)
            .on_hover_text(rom_help);
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
        render_joystick_mappings(ui, editor, &available, active, active_mappings);
        if let Err(error) = editor
            .catalog
            .validate(&editor.catalog.configurations[index])
        {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_library_setting(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        ui.strong("Application settings");
        ui.label("Game Library directory");
        if let Some(root) = &editor.library_root {
            ui.label(root.display().to_string());
        } else {
            ui.label("No library directory selected");
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Choose library directory...").clicked() {
                let mut picker = rfd::FileDialog::new();
                if let Some(directory) = editor.library_root.as_ref().filter(|path| path.is_dir()) {
                    picker = picker.set_directory(directory);
                }
                if let Some(directory) = picker.pick_folder() {
                    editor.library_root = Some(directory);
                    editor.error = None;
                }
            }
            if editor.library_root != self.player.library_root
                && ui.button("Keep current directory").clicked()
            {
                editor.library_root.clone_from(&self.player.library_root);
                editor.error = None;
            }
        });
        if let Some(error) = &self.player.library_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let Some(error) = self
            .player
            .library_sync
            .as_ref()
            .and_then(|sync| sync.read_error.as_deref())
        {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Could not read library: {error}"),
            );
        }
        ui.separator();
    }
}

fn hardware_settings(ui: &mut egui::Ui, launch: &mut crate::player_config::LaunchConfiguration) {
    use config::{A500Preset, VideoStandard};

    let hardware = |preset| match preset {
        A500Preset::Bare512k => ("A500 512 KB", "512 KB Chip RAM. No battery-backed clock."),
        A500Preset::Standard1Mb => (
            "A500 1 MB",
            "512 KB Chip RAM and 512 KB Slow RAM with a battery-backed clock.",
        ),
        A500Preset::ExpandedPowerUser => (
            "A500 5 MB",
            "512 KB Chip RAM, 512 KB Slow RAM and 4 MB Fast RAM with a battery-backed clock.",
        ),
    };
    let video = |standard| {
        match standard {
        VideoStandard::Pal => ("PAL", "50 Hz nominal video timing, 312 lines per non-interlaced short field and a 3.546895 MHz color clock."),
        VideoStandard::Ntsc => ("NTSC", "60 Hz nominal video timing, 262 lines per non-interlaced short field and a 3.579545 MHz color clock."),
    }
    };
    ui.label("Hardware, video, ROM and drive changes take effect on the next launch.");
    let label = ui.label("Hardware");
    egui::ComboBox::from_id_salt("hardware_preset")
        .selected_text(hardware(launch.preset).0)
        .show_ui(ui, |ui| {
            for preset in [
                A500Preset::Bare512k,
                A500Preset::Standard1Mb,
                A500Preset::ExpandedPowerUser,
            ] {
                let (name, summary) = hardware(preset);
                ui.selectable_value(&mut launch.preset, preset, name)
                    .on_hover_text(summary);
            }
        })
        .response
        .labelled_by(label.id)
        .on_hover_ui(|ui| {
            ui.strong(hardware(launch.preset).0);
            ui.label(hardware(launch.preset).1);
            ui.label("Configure attached floppy drives separately below.");
        });
    ui.small(hardware(launch.preset).1);
    let label = ui.label("Video");
    egui::ComboBox::from_id_salt("video_standard")
        .selected_text(video(launch.video).0)
        .show_ui(ui, |ui| {
            for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
                let (name, timing) = video(standard);
                ui.selectable_value(&mut launch.video, standard, name)
                    .on_hover_text(timing);
            }
        })
        .response
        .labelled_by(label.id)
        .on_hover_ui(|ui| {
            ui.strong(video(launch.video).0);
            ui.label(video(launch.video).1);
            ui.label("Choose the timing expected by the Amiga software.");
        });
    ui.small(video(launch.video).1);
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
    active_mappings: &[JoystickMapping],
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
                    let session = active.filter(|configuration| {
                        configuration.controls.uses_mapping(&mapping.id)
                            && active_mappings
                                .iter()
                                .any(|retained| retained.id == mapping.id)
                    });
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
