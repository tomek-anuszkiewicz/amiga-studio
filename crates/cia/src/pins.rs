//! Delivered connector levels, separate from the unfinished serial engine.

use crate::Cia;
use serde::{Deserialize, Serialize};

/// Serial and TOD levels retained until the next input sampling phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiaInputs {
    pub sp_high: bool,
    pub cnt_high: bool,
    pub tod_high: bool,
}

/// TTL pin levels: true is high, including released open-drain outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiaPins {
    pub incoming: Option<CiaInputs>,
    /// Only connected sources capture these inputs; disconnected levels persist.
    pub incoming_port_a: Option<u8>,
    pub incoming_flag_high: Option<bool>,
    pub port_a_input: u8,
    pub port_b_input: u8,
    pub sp_input_high: bool,
    pub cnt_input_high: bool,
    pub sp_output_high: bool,
    pub cnt_output_high: bool,
    pub pc_high: bool,
    pub flag_high: bool,
    /// Delivered TOD clock level; positive transitions count one event.
    pub tod_high: bool,
}

impl Default for CiaPins {
    fn default() -> Self {
        Self {
            incoming: None,
            incoming_port_a: None,
            incoming_flag_high: None,
            port_a_input: u8::MAX,
            port_b_input: u8::MAX,
            sp_input_high: true,
            cnt_input_high: true,
            sp_output_high: true,
            cnt_output_high: true,
            pc_high: true,
            flag_high: true,
            tod_high: true,
        }
    }
}

impl Cia {
    /// Released inputs pull high; outputs drive their committed Port A latch.
    #[inline]
    pub fn port_a_output_pins(&self) -> u8 {
        self.pra | !self.ddra
    }

    /// TOD is edge-sensitive, independently of software register-write capture.
    pub fn receive_tod_pin(&mut self, high: bool) {
        if !self.pins.tod_high && high {
            self.tick_tod();
        }
        self.pins.tod_high = high;
    }

    /// Drives SP/CNT together. Serial shifting belongs to the CIA engine in 2.4.
    pub fn receive_serial_pins(&mut self, sp_high: bool, cnt_high: bool) {
        self.pins.sp_input_high = sp_high;
        self.pins.cnt_input_high = cnt_high;
    }

    /// Captures the active-low FLAG input; a held low level is not another edge.
    pub fn receive_flag_pin(&mut self, high: bool) {
        if self.pins.flag_high && !high {
            self.trigger_flag_pin();
        }
        self.pins.flag_high = high;
    }

    /// Drives external Port A levels independently of the output latch/direction.
    #[inline]
    pub fn set_input_pins_a(&mut self, pins: u8, mask: u8) {
        self.pins.port_a_input = (self.pins.port_a_input & !mask) | (pins & mask);
    }

    /// Polls whether Port A output data was written, clearing the flag and returning the written byte
    #[inline]
    pub fn poll_pra_output(&mut self) -> Option<u8> {
        if self.pra_mutated {
            self.pra_mutated = false;
            Some(self.pra)
        } else {
            None
        }
    }

    /// Polls a Port B latch/direction change and returns its effective pulled-up pin levels
    #[inline]
    pub fn poll_prb_output(&mut self) -> Option<u8> {
        if self.prb_mutated {
            self.prb_mutated = false;
            Some(self.prb | !self.ddrb)
        } else {
            None
        }
    }
}

impl Cia {
    /// Samples retained inputs before engine work; source changes cannot alter them.
    pub fn sample_input_pins(&mut self) {
        if let Some(pins) = self.pins.incoming_port_a.take() {
            self.set_input_pins_a(pins, u8::MAX);
        }
        if let Some(high) = self.pins.incoming_flag_high.take() {
            self.receive_flag_pin(high);
        }
        if let Some(input) = self.pins.incoming.take() {
            self.receive_serial_pins(input.sp_high, input.cnt_high);
            self.receive_tod_pin(input.tod_high);
        }
    }
}
