//! Amiga Dual Controller Game Ports Subsystem
//!
//! Models the two physical 9-pin Atari D-Sub controller ports (Port 1 and Port 2)
//! on the Commodore Amiga 500. Provides pluggable device slots (`Mouse`, `Joystick`,
//! `None`), custom chip signal decoding for Denise (`JOY0DAT`, `JOY1DAT`),
//! Paula (`POT0DAT`, `POT1DAT`, `POTGOR`), and CIA-A (`PRA` bits 6 & 7),
//! as well as host input event routing.

pub use joystick::{self, Joystick};
pub use mouse::{self, Mouse};

use serde::{Deserialize, Serialize};

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

    /// Returns the 16-bit JOYxDAT register value for Denise
    #[inline]
    pub fn joy_dat(&self) -> u16 {
        match self {
            PortDevice::None => 0x0000,
            PortDevice::Mouse(ref m) => m.joy_dat(),
            PortDevice::Joystick(ref j) => j.joy_dat(),
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GamePorts {
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

    // --- Denise Chipset Decoders ---

    /// Returns 16-bit JOY0DAT ($DFF00A) register value for Denise (Port 1)
    #[inline]
    pub fn joy0dat(&self) -> u16 {
        self.state.port1.joy_dat()
    }

    /// Returns 16-bit JOY1DAT ($DFF00C) register value for Denise (Port 2)
    #[inline]
    pub fn joy1dat(&self) -> u16 {
        self.state.port2.joy_dat()
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

    /// Computes the 16-bit POTGOR ($DFF016) read register value
    ///
    /// Combines the Paula POTGO write latch ($DFF034) with active controller pull-downs:
    /// - Bit 14: Port 2 Pin 9 (Right mouse / Fire 2). Active low (0 = pressed, 1 = released).
    /// - Bit 12: Port 2 Pin 5 (Middle mouse). Active low (0 = pressed, 1 = released).
    /// - Bit 10: Port 1 Pin 9 (Right mouse / Fire 2). Active low (0 = pressed, 1 = released).
    /// - Bit 8:  Port 1 Pin 5 (Middle mouse). Active low (0 = pressed, 1 = released).
    #[inline]
    pub fn potgor(&self, potgo_latch: u16) -> u16 {
        let mut val = potgo_latch | 0x5500; // Pull unused input lines high by default

        // Port 2 pin 9 (Bit 14)
        if self.state.port2.is_fire2_pressed() {
            val &= !(1 << 14);
        } else {
            val |= 1 << 14;
        }

        // Port 2 pin 5 (Bit 12)
        if self.state.port2.is_middle_button_pressed() {
            val &= !(1 << 12);
        } else {
            val |= 1 << 12;
        }

        // Port 1 pin 9 (Bit 10)
        if self.state.port1.is_fire2_pressed() {
            val &= !(1 << 10);
        } else {
            val |= 1 << 10;
        }

        // Port 1 pin 5 (Bit 8)
        if self.state.port1.is_middle_button_pressed() {
            val &= !(1 << 8);
        } else {
            val |= 1 << 8;
        }

        val
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

    // --- Host Input Routing Helpers ---

    /// Applies relative mouse movement deltas to whichever port holds a mouse (default: Port 1)
    pub fn apply_mouse_delta(&mut self, dx: i32, dy: i32) {
        if let PortDevice::Mouse(ref mut m) = self.state.port1 {
            m.move_rel(dx, dy);
        } else if let PortDevice::Mouse(ref mut m) = self.state.port2 {
            m.move_rel(dx, dy);
        }
    }

    /// Sets mouse button states on whichever port holds a mouse (default: Port 1)
    pub fn set_mouse_buttons(&mut self, left: bool, right: bool, middle: bool) {
        if let PortDevice::Mouse(ref mut m) = self.state.port1 {
            m.set_left_button(left);
            m.set_right_button(right);
            m.set_middle_button(middle);
        } else if let PortDevice::Mouse(ref mut m) = self.state.port2 {
            m.set_left_button(left);
            m.set_right_button(right);
            m.set_middle_button(middle);
        }
    }

    /// Sets joystick directional switches and fire buttons on whichever port holds a joystick (default: Port 2)
    pub fn set_joystick(
        &mut self,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        fire1: bool,
        fire2: bool,
    ) {
        if let PortDevice::Joystick(ref mut j) = self.state.port2 {
            j.set_directions(up, down, left, right);
            j.set_fire1(fire1);
            j.set_fire2(fire2);
        } else if let PortDevice::Joystick(ref mut j) = self.state.port1 {
            j.set_directions(up, down, left, right);
            j.set_fire1(fire1);
            j.set_fire2(fire2);
        }
    }
}
