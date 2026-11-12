//! Buffered, MSB-first 8520 serial transfer through SP and CNT.

use crate::registers::BIT_ICR_SERIAL;
use crate::Cia;
use serde::{Deserialize, Serialize};

const SERIAL_BITS: u8 = u8::BITS as u8;
const SERIAL_MSB: u8 = 1 << (SERIAL_BITS - 1);

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SerialState {
    shift: u8,
    bits: u8,
    pub pending: bool,
}

impl Cia {
    pub(super) fn reset_serial_transfer(&mut self) {
        self.serial = SerialState::default();
        // Output starts from the cleared shifter; input releases the driver.
        self.pins.sp_output_high = !self.is_sdr_output();
        self.pins.cnt_output_high = true;
    }

    pub(super) fn shift_serial_input(&mut self) {
        self.serial.shift = (self.serial.shift << 1) | u8::from(self.pins.sp_input_high);
        self.serial.bits += 1;
        if self.serial.bits == SERIAL_BITS {
            self.sdr = self.serial.shift;
            self.serial.bits = 0;
            self.trigger_icr(BIT_ICR_SERIAL);
        }
    }

    pub(super) fn clock_serial_output(&mut self) {
        if !self.is_sdr_output() {
            return;
        }
        if self.pins.cnt_output_high {
            if self.serial.bits == 0 {
                if !self.serial.pending {
                    return;
                }
                self.serial.shift = self.sdr;
                self.serial.bits = SERIAL_BITS;
                self.serial.pending = false;
            }
            self.pins.sp_output_high = self.serial.shift & SERIAL_MSB != 0;
            self.pins.cnt_output_high = false;
        } else {
            self.pins.cnt_output_high = true;
            self.serial.shift <<= 1;
            self.serial.bits -= 1;
            if self.serial.bits == 0 {
                self.trigger_icr(BIT_ICR_SERIAL);
            }
        }
    }
}
