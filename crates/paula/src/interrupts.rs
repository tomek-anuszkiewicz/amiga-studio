//! Paula interrupt latches and the HRM chapter 7 source priority encoder.

use crate::register_bits::intreq;
use serde::{Deserialize, Serialize};

const MASK_SOURCE_BITS: u16 = intreq::BIT_TBE
    | intreq::BIT_DSKBLK
    | intreq::BIT_SOFT
    | intreq::BIT_PORTS
    | intreq::BIT_COPER
    | intreq::BIT_VERTB
    | intreq::BIT_BLIT
    | intreq::BIT_AUD0
    | intreq::BIT_AUD1
    | intreq::BIT_AUD2
    | intreq::BIT_AUD3
    | intreq::BIT_RBF
    | intreq::BIT_DSKSYN
    | intreq::BIT_EXTER;
const MASK_REGISTER_BITS: u16 = MASK_SOURCE_BITS | intreq::BIT_INTEN;
/// Figure 7-4, ordered by INTREQ source bit from TBE through EXTER.
const PRIORITY_BY_SOURCE: [u8; 14] = [1, 1, 1, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 6];

fn update_latch(latch: &mut u16, write: u16) {
    let selected = write & MASK_REGISTER_BITS;
    *latch = if write & intreq::BIT_SET_CLR == 0 {
        *latch & !selected
    } else {
        *latch | selected
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptController {
    pub intena: u16,
    pub intreq: u16,
}

impl InterruptController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn write_intena(&mut self, value: u16) {
        update_latch(&mut self.intena, value);
    }

    pub fn write_intreq(&mut self, value: u16) {
        update_latch(&mut self.intreq, value);
    }

    pub fn request(&mut self, sources: u16) {
        self.intreq |= sources & MASK_SOURCE_BITS;
    }

    pub fn is_master_enabled(&self) -> bool {
        self.intena & intreq::BIT_INTEN != 0
    }

    pub fn pending_mask(&self) -> u16 {
        if self.is_master_enabled() {
            self.intena & self.intreq & MASK_SOURCE_BITS
        } else {
            0
        }
    }

    pub fn pending_level(&self) -> u8 {
        let pending = self.pending_mask();
        if pending == 0 {
            return 0;
        }
        let source = (u16::BITS - 1 - pending.leading_zeros()) as usize;
        PRIORITY_BY_SOURCE[source]
    }

    pub fn read_intenar(&self) -> u16 {
        self.intena
    }

    pub fn read_intreqr(&self) -> u16 {
        self.intreq
    }
}
