//! Separate configuration and reusable mapping editor.

use crate::player_config::{InputSource, JoystickMapping, NamedConfiguration};
use crate::EmulatorApp;
use debugger::launch::DeviceKind;

impl EmulatorApp {
    pub(crate) fn render_configuration_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurations");
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        let catalog = &mut editor.catalog;
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
                    DeviceKind::Joystick => InputSource::Keyboard("arrows".into()),
                };
            }
            if binding.device == DeviceKind::Joystick {
                egui::ComboBox::from_id_salt(("port_source", port))
                    .selected_text(match &binding.source {
                        InputSource::Keyboard(id) => format!("Keyboard: {id}"),
                        InputSource::Gamepad { identity, .. } => format!("Controller: {identity}"),
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
                        for pad in &self.player.gamepads {
                            let mapping = catalog
                                .mappings
                                .first()
                                .map_or(String::new(), |m| m.id.clone());
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
                    if !self.player.gamepads.iter().any(|p| &p.identity == identity) {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "Controller unavailable; select a current connection",
                        );
                    }
                    egui::ComboBox::from_id_salt(("pad_mapping", port))
                        .selected_text(mapping.as_str())
                        .show_ui(ui, |ui| {
                            for candidate in &catalog.mappings {
                                ui.selectable_value(mapping, candidate.id.clone(), &candidate.name);
                            }
                        });
                }
            }
        }
        ui.separator();
        ui.strong("Frontend keys");
        ui.small("F12 switches Game/Debug. The middle mouse button always releases capture.");
        for (label, key) in [
            (
                "Mouse release key (optional)",
                &mut configuration.controls.host.frontend.release,
            ),
            (
                "Pause / Resume",
                &mut configuration.controls.host.frontend.pause,
            ),
            (
                "Save progress",
                &mut configuration.controls.host.frontend.save,
            ),
            (
                "Load progress",
                &mut configuration.controls.host.frontend.load,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.text_edit_singleline(key).on_hover_text(
                    "Use a key name such as F5, F6 or F9. Leave release empty to use only the middle mouse button.",
                );
            });
        }
        ui.collapsing("Controller frontend shortcuts", |ui| {
            if ui.button("Add controller shortcut").clicked() {
                configuration.controls.host.gamepad_shortcuts.push(
                    crate::player_config::GamepadShortcut {
                        identity: self
                            .player
                            .gamepads
                            .first()
                            .map_or(String::new(), |p| p.identity.clone()),
                        button: crate::player_config::PAD_START,
                        action: crate::player_config::FrontendAction::Release,
                    },
                );
            }
            let mut remove = None;
            for (index, shortcut) in configuration
                .controls
                .host
                .gamepad_shortcuts
                .iter_mut()
                .enumerate()
            {
                ui.push_id(index, |ui| {
                    if ui.button("Remove shortcut").clicked() {
                        remove = Some(index);
                    }
                    egui::ComboBox::from_id_salt("action_pad")
                        .selected_text(&shortcut.identity)
                        .show_ui(ui, |ui| {
                            for pad in &self.player.gamepads {
                                ui.selectable_value(
                                    &mut shortcut.identity,
                                    pad.identity.clone(),
                                    &pad.name,
                                );
                            }
                        });
                    ui.add(
                        egui::DragValue::new(&mut shortcut.button)
                            .range(0..=crate::player_config::PAD_BUTTON_COUNT - 1),
                    );
                    egui::ComboBox::from_id_salt("pad_action")
                        .selected_text(format!("{:?}", shortcut.action))
                        .show_ui(ui, |ui| {
                            use crate::player_config::FrontendAction;
                            for action in [
                                FrontendAction::Release,
                                FrontendAction::Pause,
                                FrontendAction::Save,
                                FrontendAction::Load,
                                FrontendAction::Debug,
                            ] {
                                ui.selectable_value(
                                    &mut shortcut.action,
                                    action,
                                    format!("{action:?}"),
                                );
                            }
                        });
                });
            }
            if let Some(index) = remove {
                configuration.controls.host.gamepad_shortcuts.remove(index);
            }
        });
        ui.collapsing("Amiga keyboard remapping", |ui| {
            ui.label("Raw physical position and guest keycode (hex). Amiga keys: left 66, right 67, Ctrl 63, left/right Alt 64/65.");
            let mut remove = None;
            for (index,(host,guest)) in configuration.controls.host.keyboard.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(host).hexadecimal(2,false,true).range(0..=crate::host_keys::MAX_AMIGA_KEYCODE));
                    ui.label("to");
                    ui.add(egui::DragValue::new(guest).hexadecimal(2,false,true).range(0..=crate::host_keys::MAX_AMIGA_KEYCODE));
                    if ui.button("Remove override").clicked() {remove=Some(index);}
                });
            }
            if let Some(index)=remove {configuration.controls.host.keyboard.remove(index);}
            if ui.button("Add key override").clicked() {configuration.controls.host.keyboard.push((keyboard::SCANCODE_L_AMIGA,keyboard::SCANCODE_L_AMIGA));}
        });
        ui.separator();
        ui.heading("Reusable joystick mappings");
        ui.label("Keys use physical Amiga positions in hex; controller buttons and axes use standard host indices.");
        if ui.button("Create mapping").clicked() {
            let id = format!("mapping-{}", catalog.mappings.len());
            catalog.mappings.push(JoystickMapping {
                id,
                name: "New mapping".into(),
                keys: [
                    crate::host_keys::SCANCODE_KEY_W,
                    crate::host_keys::SCANCODE_KEY_S,
                    crate::host_keys::SCANCODE_KEY_A,
                    crate::host_keys::SCANCODE_KEY_D,
                    crate::host_keys::SCANCODE_SPACE,
                    crate::host_keys::SCANCODE_KEY_C,
                ],
                ..JoystickMapping::default()
            });
        }
        for mapping in &mut catalog.mappings {
            ui.push_id(&mapping.id, |ui| {
                ui.collapsing(mapping.name.clone(), |ui| {
                    ui.text_edit_singleline(&mut mapping.name);
                    for (index, label) in ["Up", "Down", "Left", "Right", "Fire 1", "Fire 2"]
                        .into_iter()
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            ui.add(
                                egui::DragValue::new(&mut mapping.keys[index])
                                    .hexadecimal(2, false, true)
                                    .range(0..=crate::host_keys::MAX_AMIGA_KEYCODE),
                            );
                            ui.label("Controller button");
                            ui.add(
                                egui::DragValue::new(&mut mapping.buttons[index])
                                    .range(0..=crate::player_config::PAD_BUTTON_COUNT - 1),
                            );
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label("X / Y axis");
                        for axis in &mut mapping.axes {
                            ui.add(
                                egui::DragValue::new(axis)
                                    .range(0..=crate::player_config::MAX_CONTROLLER_AXIS),
                            );
                        }
                    });
                    ui.add(
                        egui::Slider::new(
                            &mut mapping.threshold,
                            crate::player_config::MIN_CONTROLLER_THRESHOLD
                                ..=crate::player_config::MAX_CONTROLLER_THRESHOLD,
                        )
                        .text("Axis threshold / deadzone"),
                    );
                });
            });
        }
        let configuration = &catalog.configurations[index];
        if let Err(error) = catalog.validate(configuration) {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}
