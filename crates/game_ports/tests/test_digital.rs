use game_ports::{GamePorts, Mouse, PortDevice};

#[test]
fn every_pot_line_resolves_direction_data_and_button_pull_down() {
    for bit in [0x0100, 0x0400, 0x1000, 0x4000] {
        for output in [false, true] {
            for high in [false, true] {
                for pressed in [false, true] {
                    let mut ports = GamePorts::new();
                    let mut mouse = Mouse::new();
                    mouse.set_middle_button(pressed && bit & 0x1100 != 0);
                    mouse.set_right_button(pressed && bit & 0x4400 != 0);
                    if bit & 0x0500 != 0 {
                        ports.plug_port1(PortDevice::Mouse(mouse));
                    } else {
                        ports.plug_port2(PortDevice::Mouse(mouse));
                    }
                    ports.pins.pot_output_mask = if output { bit } else { 0 };
                    ports.pins.pot_output_high = if high { bit } else { 0 };
                    assert_eq!(ports.potgor() & bit != 0, !pressed && (!output || high));
                    assert_eq!(ports.potgor() & !bit, 0x5500 & !bit);
                }
            }
        }
    }
    let mut ports = GamePorts::new();
    ports.plug_port1(PortDevice::None);
    ports.plug_port2(PortDevice::None);
    ports.pins.pot_output_mask = 0x5500;
    assert_eq!(ports.potgor(), 0);
    ports.pins.fire_output_mask = 0xc0;
    ports.pins.fire_output_high = 0;
    assert_eq!(ports.sample_ciaa_port_a_inputs(), 0);
}

#[test]
fn mouse_outputs_one_gray_transition_per_count_and_reverses_direction() {
    let mut ports = GamePorts::new();
    ports.apply_mouse_delta(4, 0);
    for pins in [0x0d, 0x05, 0x07, 0x0f] {
        ports.step_cck();
        assert_eq!(ports.pins.direction_pins[0], pins);
    }
    ports.apply_mouse_delta(0, -4);
    for pins in [0x0b, 0x0a, 0x0e, 0x0f] {
        ports.step_cck();
        assert_eq!(ports.pins.direction_pins[0], pins);
    }
    ports.plug_port1(PortDevice::None);
    ports.step_cck();
    assert_eq!(ports.pins.direction_pins, [0x0f; 2]);
}
