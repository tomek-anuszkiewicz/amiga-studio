//! Amiga RS-232 Serial Port Subsystem
//!
//! Models the Paula UART transceiver, baud rate divisor (SERPER),
//! transmit/receive registers (SERDAT/SERDATR), and CIA-B RS-232 handshakes.

use serde::{Deserialize, Serialize};

/// RS-232 Serial Port transceiver and interface
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SerialPort {
    /// Serial data and status read register (SERDATR)
    pub serdatr: u16,
    /// Serial data write holding register (SERDAT)
    pub serdat: u16,
    /// Serial period divisor and framing register (SERPER)
    pub serper: u16,
    /// Clear To Send handshake line (active low, from CIA-B)
    pub cts: bool,
    /// Request To Send handshake line (active low, to CIA-B)
    pub rts: bool,
    /// Data Set Ready line
    pub dsr: bool,
    /// Carrier Detect line
    pub cd: bool,
}

impl SerialPort {
    /// Creates a new serial port instance with empty buffers and TBE asserted
    pub fn new() -> Self {
        let mut port = Self::default();
        // Bit 13: TBE (Transmitter Buffer Empty) defaults asserted
        // Bit 12: TSRE (Transmitter Shift Register Empty) defaults asserted
        port.serdatr = 0x3000;
        port
    }

    /// Resets serial port registers
    pub fn reset(&mut self) {
        self.serdatr = 0x3000;
        self.serdat = 0;
        self.serper = 0;
        self.cts = false;
        self.rts = false;
        self.dsr = false;
        self.cd = false;
    }

    /// Writes to SERDAT register
    #[inline]
    pub fn write_serdat(&mut self, val: u16) {
        self.serdat = val;
        // Transmitter buffer is now loaded (clear TBE bit 13)
        self.serdatr &= !0x2000;
    }

    /// Writes to SERPER register
    #[inline]
    pub fn write_serper(&mut self, val: u16) {
        self.serper = val;
    }

    /// Advances the serial port state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: UART shift clock progression
    }
}
