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

#[test]
fn restored_mouse_phases_normalize_with_idle_and_moving_axes(
) -> Result<(), Box<dyn std::error::Error>> {
    let x_pins = [0x0a, 0x08, 0x00, 0x02];
    let y_pins = [0x05, 0x04, 0x00, 0x01];
    for x in [0u8, 1, 2, 3, 4, 255] {
        for y in [0u8, 1, 2, 3, 4, 255] {
            for dx in [-1i32, 0, 1] {
                for dy in [-1i32, 0, 1] {
                    let mut saved = serde_json::to_value(Mouse::new())?;
                    saved["phase_x"] = x.into();
                    saved["phase_y"] = y.into();
                    let mut mouse: Mouse = serde_json::from_value(saved)?;
                    mouse.move_rel(dx, dy);
                    let expected_x = (i32::from(x) + dx).rem_euclid(4) as usize;
                    let expected_y = (i32::from(y) + dy).rem_euclid(4) as usize;
                    assert_eq!(mouse.step_cck(), x_pins[expected_x] | y_pins[expected_y]);
                    let state = serde_json::to_value(mouse)?;
                    assert_eq!(state["phase_x"], expected_x);
                    assert_eq!(state["phase_y"], expected_y);
                    assert_eq!(state["pending_x"], 0);
                    assert_eq!(state["pending_y"], 0);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn mouse_motion_saturates_then_consumes_one_count_per_cck() -> Result<(), Box<dyn std::error::Error>>
{
    let mut mouse = Mouse::new();
    mouse.move_rel(i32::MAX, i32::MIN);
    mouse.move_rel(1, -1);
    assert_eq!(mouse.step_cck(), 0x09);
    let state = serde_json::to_value(mouse)?;
    assert_eq!(state["pending_x"], i32::MAX - 1);
    assert_eq!(state["pending_y"], i32::MIN + 1);
    Ok(())
}
