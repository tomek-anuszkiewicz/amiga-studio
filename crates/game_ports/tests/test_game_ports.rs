//! Unit and Integration Tests for Amiga Game Ports Subsystem
//!
//! Validates pluggable port configuration, Denise JOYxDAT decoding,
//! Paula POTGOR right/middle button decoding, CIA-A fire button decoding,
//! and Serde snapshot serialization.

use game_ports::{GamePorts, GamePortsState, Joystick, PortDevice};

#[test]
fn test_default_ports_configuration() {
    let gp = GamePorts::new();

    // Port 1 defaults to Mouse
    match gp.port1() {
        PortDevice::Mouse(m) => {
            assert_eq!(m.x, 0);
            assert_eq!(m.y, 0);
            assert!(!m.left_button);
            assert!(!m.right_button);
            assert!(!m.middle_button);
        }
        _ => panic!("Expected Port 1 to default to Mouse"),
    }

    // Port 2 defaults to Joystick
    match gp.port2() {
        PortDevice::Joystick(j) => {
            assert!(!j.up);
            assert!(!j.down);
            assert!(!j.left);
            assert!(!j.right);
            assert!(!j.fire1);
            assert!(!j.fire2);
        }
        _ => panic!("Expected Port 2 to default to Joystick"),
    }
}

#[test]
fn test_mouse_motion_and_denise_joy0dat() {
    let mut gp = GamePorts::new();
    assert_eq!(gp.joy0dat(), 0x0000);

    // Apply relative motion (+10 X, +20 Y)
    gp.apply_mouse_delta(10, 20);
    assert_eq!(gp.joy0dat(), (20 << 8) | 10);

    // Further motion with 8-bit counter wrapping
    gp.apply_mouse_delta(250, 0); // 10 + 250 = 260 -> wraps to 4 (u8)
    assert_eq!(gp.joy0dat(), (20 << 8) | 4);
}

#[test]
fn test_mouse_buttons_and_chipset_decoding() {
    let mut gp = GamePorts::new();

    // Initially all released
    assert!(!gp.fire1_port1());
    // In POTGOR, unpressed pins are pulled high (1)
    let potgo_idle = gp.potgor(0x0000);
    assert_ne!(
        potgo_idle & (1 << 10),
        0,
        "Pin 9 (Right button) should be high when released"
    );
    assert_ne!(
        potgo_idle & (1 << 8),
        0,
        "Pin 5 (Middle button) should be high when released"
    );

    // Press left mouse button (CIA-A PRA bit 6)
    gp.set_mouse_buttons(true, false, false);
    assert!(gp.fire1_port1());

    // Press right mouse button (Paula POTGOR bit 10 pulled low)
    gp.set_mouse_buttons(false, true, false);
    assert!(!gp.fire1_port1());
    let potgo_right = gp.potgor(0x0000);
    assert_eq!(
        potgo_right & (1 << 10),
        0,
        "Pin 9 (Right button) must be pulled low (0) when pressed"
    );
    assert_ne!(
        potgo_right & (1 << 8),
        0,
        "Pin 5 (Middle button) must stay high"
    );

    // Press middle mouse button (Paula POTGOR bit 8 pulled low)
    gp.set_mouse_buttons(false, false, true);
    let potgo_middle = gp.potgor(0x0000);
    assert_eq!(
        potgo_middle & (1 << 8),
        0,
        "Pin 5 (Middle button) must be pulled low (0) when pressed"
    );
    assert_ne!(
        potgo_middle & (1 << 10),
        0,
        "Pin 9 (Right button) must stay high"
    );
}

#[test]
fn test_joystick_direction_and_fire_decoding() {
    let mut gp = GamePorts::new();

    // Directional encoding test (XOR format in JOY1DAT):
    // UP (bit 8 = UP^LEFT = 1^0 = 1, bit 9 = LEFT = 0) -> 0x0100
    gp.set_joystick(true, false, false, false, false, false);
    assert_eq!(gp.joy1dat(), 0x0100);

    // RIGHT (bit 1 = RIGHT = 1, bit 0 = DOWN^RIGHT = 0^1 = 1) -> 0x0003
    gp.set_joystick(false, false, false, true, false, false);
    assert_eq!(gp.joy1dat(), 0x0003);

    // Primary fire on Port 2 (CIA-A PRA bit 7)
    gp.set_joystick(false, false, false, false, true, false);
    assert!(gp.fire1_port2());

    // Secondary fire on Port 2 (Paula POTGOR bit 14 pulled low)
    gp.set_joystick(false, false, false, false, false, true);
    let potgo_fire2 = gp.potgor(0x0000);
    assert_eq!(
        potgo_fire2 & (1 << 14),
        0,
        "Port 2 fire 2 must pull bit 14 low"
    );
}

#[test]
fn test_port_device_swapping_and_multiplayer() {
    let mut gp = GamePorts::new();

    // Swap to 2 Joysticks (2-player multiplayer games like Lotus, Sensible Soccer)
    gp.plug_port1(PortDevice::Joystick(Joystick::new()));
    gp.plug_port2(PortDevice::Joystick(Joystick::new()));

    // Control Joystick 1
    if let PortDevice::Joystick(ref mut j1) = gp.port1_mut() {
        j1.set_directions(true, false, false, false);
        j1.set_fire1(true);
    }
    assert_eq!(gp.joy0dat(), 0x0100);
    assert!(gp.fire1_port1());

    // Control Joystick 2
    if let PortDevice::Joystick(ref mut j2) = gp.port2_mut() {
        j2.set_directions(false, false, false, true);
        j2.set_fire1(true);
    }
    assert_eq!(gp.joy1dat(), 0x0003);
    assert!(gp.fire1_port2());

    // Test disconnected port
    gp.plug_port1(PortDevice::None);
    assert_eq!(gp.joy0dat(), 0x0000);
    assert!(!gp.fire1_port1());
}

#[test]
fn test_reset_dynamics() {
    let mut gp = GamePorts::new();
    gp.apply_mouse_delta(50, 60);
    gp.set_mouse_buttons(true, true, true);
    gp.set_joystick(true, true, true, true, true, true);

    gp.reset();

    assert_eq!(gp.joy0dat(), 0x0000);
    assert_eq!(gp.joy1dat(), 0x0000);
    assert!(!gp.fire1_port1());
    assert!(!gp.fire1_port2());
}

#[test]
fn test_serde_state_roundtrip() {
    let mut gp = GamePorts::new();
    gp.apply_mouse_delta(15, 35);
    gp.set_mouse_buttons(true, false, true);

    let serialized = serde_json::to_string(&gp.state).expect("Failed to serialize GamePortsState");
    let deserialized: GamePortsState =
        serde_json::from_str(&serialized).expect("Failed to deserialize GamePortsState");

    assert_eq!(gp.state, deserialized);
}
