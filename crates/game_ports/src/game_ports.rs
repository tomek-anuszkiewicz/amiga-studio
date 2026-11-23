//! Amiga Dual Controller Game Ports Subsystem
//!
//! Models the two physical 9-pin Atari D-Sub controller ports (Port 1 and Port 2)
//! on the Commodore Amiga 500. Provides pluggable device slots (`Mouse`, `Joystick`,
//! `None`), custom chip signal decoding for Denise (`JOY0DAT`, `JOY1DAT`),
//! Paula (`POT0DAT`, `POT1DAT`, `POTGOR`), and CIA-A (`PRA` bits 6 & 7),
//! as well as host input event routing.

pub mod joystick;
pub use joystick::Joystick;
pub mod mouse;
pub use mouse::Mouse;

use serde::{Deserialize, Serialize};

mod pins;
pub use pins::{GamePortsInputs, GamePortsPins};

const BIT_POT1_FIRE2: u16 = 1 << 14;
const BIT_POT1_FIRE3: u16 = 1 << 12;
const BIT_POT0_FIRE2: u16 = 1 << 10;
const BIT_POT0_FIRE3: u16 = 1 << 8;

/// CIA-A Port A bit 6: /FIR0 (Port 1 left mouse button, active low)
pub const BIT_CIAA_PRA_FIR0: u8 = 0x40;

/// CIA-A Port A bit 7: /FIR1 (Port 2 joystick fire 1, active low)
pub const BIT_CIAA_PRA_FIR1: u8 = 0x80;

/// CIA-A Port A mask for primary fire buttons (bits 6 and 7)
pub const MASK_CIAA_PRA_FIRE: u8 = BIT_CIAA_PRA_FIR0 | BIT_CIAA_PRA_FIR1;

/// Controller device plugged into a 9-pin game port
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDevice {
    /// Disconnected / open port
    None,
    /// Amiga 2-button or 3-button quadrature mouse
    Mouse(Mouse),
    /// Standard Atari / Amiga digital joystick
    Joystick(Joystick),
}

impl PortDevice {
    /// Resets the internal state of the plugged device
    pub fn reset(&mut self) {
        match self {
            PortDevice::None => {}
            PortDevice::Mouse(ref mut m) => m.reset(),
            PortDevice::Joystick(ref mut j) => j.reset(),
        }
    }

    /// Advances device outputs, returning the four direction/quadrature pins.
    fn step_direction_pins(&mut self) -> u8 {
        match self {
            Self::None => config::mask::game_port::MASK_DIRECTION_INPUTS,
            Self::Mouse(mouse) => mouse.step_cck(),
            Self::Joystick(joystick) => joystick.direction_pins(),
        }
    }

    /// Returns true if the primary fire button / left mouse button is pressed
    #[inline]
    pub fn is_fire1_pressed(&self) -> bool {
        match self {
            PortDevice::None => false,
            PortDevice::Mouse(ref m) => m.left_button,
            PortDevice::Joystick(ref j) => j.fire1,
        }
    }

    /// Returns true if the secondary fire button / right mouse button is pressed
    #[inline]
    pub fn is_fire2_pressed(&self) -> bool {
        match self {
            PortDevice::None => false,
            PortDevice::Mouse(ref m) => m.right_button,
            PortDevice::Joystick(ref j) => j.fire2,
        }
    }

    /// Returns true if the middle mouse button is pressed
    #[inline]
    pub fn is_middle_button_pressed(&self) -> bool {
        match self {
            PortDevice::None => false,
            PortDevice::Mouse(ref m) => m.middle_button,
            PortDevice::Joystick(_) => false,
        }
    }
}

/// Decoupled hardware state snapshot for Amiga Game Ports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePortsState {
    /// Controller device connected to Port 1 (default: Mouse)
    pub port1: PortDevice,
    /// Controller device connected to Port 2 (default: Joystick)
    pub port2: PortDevice,
    /// Paula POT0DAT register latch (Port 1 analog counters)
    pub pot0dat: u16,
    /// Paula POT1DAT register latch (Port 2 analog counters)
    pub pot1dat: u16,
}

impl Default for GamePortsState {
    fn default() -> Self {
        Self {
            port1: PortDevice::Mouse(Mouse::new()),
            port2: PortDevice::Joystick(Joystick::new()),
            pot0dat: 0x0000,
            pot1dat: 0x0000,
        }
    }
}

/// Amiga Game Ports Subsystem handle
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePorts {
    /// Motherboard-facing signal state.
    pub pins: GamePortsPins,
    /// Observable hardware state snapshot
    pub state: GamePortsState,
}

impl GamePorts {
    /// Creates a new GamePorts subsystem with standard defaults (Port 1 = Mouse, Port 2 = Joystick)
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all plugged devices and registers to initial state
    pub fn reset(&mut self) {
        self.pins = GamePortsPins::default();
        self.state.port1.reset();
        self.state.port2.reset();
        self.state.pot0dat = 0;
        self.state.pot1dat = 0;
    }

    /// Plugs a device into Port 1
    #[inline]
    pub fn plug_port1(&mut self, device: PortDevice) {
        self.state.port1 = device;
    }

    /// Plugs a device into Port 2
    #[inline]
    pub fn plug_port2(&mut self, device: PortDevice) {
        self.state.port2 = device;
    }

    /// Read-only access to Port 1 device
    #[inline]
    pub fn port1(&self) -> &PortDevice {
        &self.state.port1
    }

    /// Mutable access to Port 1 device
    #[inline]
    pub fn port1_mut(&mut self) -> &mut PortDevice {
        &mut self.state.port1
    }

    /// Read-only access to Port 2 device
    #[inline]
    pub fn port2(&self) -> &PortDevice {
        &self.state.port2
    }

    /// Mutable access to Port 2 device
    #[inline]
    pub fn port2_mut(&mut self) -> &mut PortDevice {
        &mut self.state.port2
    }

    /// Advances external device waveforms before the current input sampling phase.
    #[inline]
    pub fn step_cck(&mut self) {
        self.pins.direction_pins = [
            self.state.port1.step_direction_pins(),
            self.state.port2.step_direction_pins(),
        ];
    }

    // --- Paula Chipset Decoders ---

    /// Returns 16-bit POT0DAT ($DFF012) pot counter data for Port 1
    #[inline]
    pub fn pot0dat(&self) -> u16 {
        self.state.pot0dat
    }

    /// Returns 16-bit POT1DAT ($DFF014) pot counter data for Port 2
    #[inline]
    pub fn pot1dat(&self) -> u16 {
        self.state.pot1dat
    }

    /// Resolves all four digital POT lines from delivered drivers and buttons.
    pub fn potgor(&self) -> u16 {
        use config::mask::game_port::MASK_POT_DATA;
        let mut levels = (self.pins.pot_output_high | !self.pins.pot_output_mask) & MASK_POT_DATA;
        for (pressed, bit) in [
            (self.state.port1.is_middle_button_pressed(), BIT_POT0_FIRE3),
            (self.state.port1.is_fire2_pressed(), BIT_POT0_FIRE2),
            (self.state.port2.is_middle_button_pressed(), BIT_POT1_FIRE3),
            (self.state.port2.is_fire2_pressed(), BIT_POT1_FIRE2),
        ] {
            if pressed {
                levels &= !bit;
            }
        }
        levels
    }

    // --- CIA-A Primary Fire Decoders ---

    /// Returns true if Port 1 primary trigger / left click is pressed
    ///
    /// In Amiga hardware, connected to CIA-A PRA bit 6 (/FIR0). Active low: 0 = pressed.
    #[inline]
    pub fn fire1_port1(&self) -> bool {
        self.state.port1.is_fire1_pressed()
    }

    /// Returns true if Port 2 primary trigger / fire 1 is pressed
    ///
    /// In Amiga hardware, connected to CIA-A PRA bit 7 (/FIR1). Active low: 0 = pressed.
    #[inline]
    pub fn fire1_port2(&self) -> bool {
        self.state.port2.is_fire1_pressed()
    }

    /// Samples CIA-A Port A fire button input pins (bits 6 and 7, active low).
    ///
    /// Bit 6 = /FIR0 (Port 1 left mouse button, 0 = pressed, 1 = released)
    /// Bit 7 = /FIR1 (Port 2 joystick fire 1, 0 = pressed, 1 = released)
    #[inline]
    pub fn sample_ciaa_port_a_inputs(&self) -> u8 {
        let mut fire_pins =
            (self.pins.fire_output_high | !self.pins.fire_output_mask) & MASK_CIAA_PRA_FIRE;
        if self.fire1_port1() {
            fire_pins &= !BIT_CIAA_PRA_FIR0;
        }
        if self.fire1_port2() {
            fire_pins &= !BIT_CIAA_PRA_FIR1;
        }
        fire_pins
    }

    // --- Host Input Routing Helpers ---

    /// Applies relative movement to the explicitly selected physical port.
    pub fn apply_mouse_delta(&mut self, port: usize, dx: i32, dy: i32) {
        if let Some(PortDevice::Mouse(mouse)) = self.device_mut(port) {
            mouse.move_rel(dx, dy);
        }
    }

    /// Sets mouse buttons only on the addressed device.
    pub fn set_mouse_buttons(&mut self, port: usize, left: bool, right: bool, middle: bool) {
        if let Some(PortDevice::Mouse(mouse)) = self.device_mut(port) {
            mouse.set_left_button(left);
            mouse.set_right_button(right);
            mouse.set_middle_button(middle);
        }
    }

    /// Sets the six digital joystick contacts on one physical port.
    pub fn set_joystick(&mut self, port: usize, contacts: [bool; 6]) {
        if let Some(PortDevice::Joystick(joystick)) = self.device_mut(port) {
            let [up, down, left, right, fire1, fire2] = contacts;
            joystick.set_directions(up, down, left, right);
            joystick.set_fire1(fire1);
            joystick.set_fire2(fire2);
        }
    }

    /// Returns the destination without searching or substituting another port.
    pub fn device_mut(&mut self, port: usize) -> Option<&mut PortDevice> {
        match port {
            0 => Some(&mut self.state.port1),
            1 => Some(&mut self.state.port2),
            _ => None,
        }
    }

    /// Releases the addressed mouse while preserving its quadrature phase.
    pub fn release_mouse(&mut self, port: usize) {
        if let Some(PortDevice::Mouse(mouse)) = self.device_mut(port) {
            mouse.set_left_button(false);
            mouse.set_right_button(false);
            mouse.set_middle_button(false);
            mouse.release_motion();
        }
    }

    /// Releases host controls without resetting quadrature phases or device types.
    pub fn release_controls(&mut self) {
        for port in 0..2 {
            self.release_mouse(port);
            self.set_joystick(port, [false; 6]);
        }
    }
}
