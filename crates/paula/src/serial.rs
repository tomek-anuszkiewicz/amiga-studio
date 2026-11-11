//! Amiga RS-232 Serial Port Subsystem (Paula UART)
//!
//! Models the Paula UART transceiver, baud rate divisor (SERPER),
//! transmit/receive registers (SERDAT/SERDATR) with an unconnected serial port.

use serde::{Deserialize, Serialize};

const SERDATR_TBE: u16 = 0x2000;
const SERDATR_TSRE: u16 = 0x1000;
const SERDATR_RXD: u16 = 0x0800;

/// RS-232 Serial Port transceiver and interface
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SerialPort {
    /// Serial data and status read register (SERDATR)
    pub serdatr: u16,
    /// Serial data write holding register (SERDAT)
    pub serdat: u16,
    /// Serial period divisor and framing register (SERPER)
    pub serper: u16,
}

impl SerialPort {
    /// Creates a new serial port instance with empty buffers and TBE asserted
    pub fn new() -> Self {
        let mut port = Self::default();
        // Bit 13: TBE (Transmitter Buffer Empty) defaults asserted
        // Bit 12: TSRE (Transmitter Shift Register Empty) defaults asserted
        // Bit 11: RXD remains idle high with no attached device.
        port.serdatr = SERDATR_TBE | SERDATR_TSRE | SERDATR_RXD;
        port
    }

    /// Resets serial port registers
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Writes to SERDAT register
    #[inline]
    pub fn write_serdat(&mut self, val: u16) {
        self.serdat = val;
        // Transmitter buffer is now loaded (clear TBE bit 13)
        self.serdatr &= !SERDATR_TBE;
    }

    /// Writes to SERPER register
    #[inline]
    pub fn write_serper(&mut self, val: u16) {
        self.serper = val;
    }

    /// Advances the serial port state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // UART shift clock is not implemented; see ROADMAP.md#chipset-reconstruction.
    }
}
