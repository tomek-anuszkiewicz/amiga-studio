//! HRM chapter 6 bus sequences and cross-word shifting.

use crate::register_bits::{blitter_fields as field, bltcon};
use serde::{Deserialize, Serialize};

pub const MAX_WORD_PHASES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlitterPhase {
    #[default]
    Idle,
    FetchA,
    FetchB,
    FetchC,
    WriteD,
    BusIdle,
}

impl BlitterPhase {
    #[inline]
    pub(super) fn requires_bus(self) -> bool {
        matches!(
            self,
            Self::FetchA | Self::FetchB | Self::FetchC | Self::WriteD
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingBlitWord {
    pub data: u16,
    pub last_in_row: bool,
}

/// Shifting continues across word and row boundaries, in the selected direction.
pub fn barrel_shift(current: u16, previous: u16, shift: u16, descending: bool) -> u16 {
    if descending {
        ((u32::from(current) << shift) | (u32::from(previous) >> (16 - shift))) as u16
    } else {
        (((u32::from(previous) << 16) | u32::from(current)) >> shift) as u16
    }
}

const CHANNEL_COMBINATIONS: usize = 16;
const BIT_CHANNEL_SHIFT: u32 = bltcon::BIT_USED.trailing_zeros();
const MASK_CHANNELS: u16 =
    bltcon::BIT_USEA | bltcon::BIT_USEB | bltcon::BIT_USEC | bltcon::BIT_USED;

// HRM Table 6-2, indexed by the contiguous A/B/C/D enables in BLTCON0.
const WORD_PHASES: [[([BlitterPhase; MAX_WORD_PHASES], u8); CHANNEL_COMBINATIONS]; 2] = {
    use BlitterPhase::{BusIdle as I, FetchA as A, FetchB as B, FetchC as C, WriteD as D};
    let normal = [
        ([I, I, I, I], 2),
        ([I, D, I, I], 2),
        ([I, C, I, I], 2),
        ([I, C, D, I], 3),
        ([I, B, I, I], 3),
        ([I, B, D, I], 3),
        ([I, B, C, I], 3),
        ([I, B, C, D], 4),
        ([A, I, I, I], 2),
        ([A, D, I, I], 2),
        ([A, C, I, I], 2),
        ([A, C, D, I], 3),
        ([A, B, I, I], 3),
        ([A, B, D, I], 3),
        ([A, B, C, I], 3),
        ([A, B, C, D], 4),
    ];
    let mut fill = normal;
    let mut channel = 0;
    while channel < CHANNEL_COMBINATIONS {
        let enables = (channel as u16) << BIT_CHANNEL_SHIFT;
        if enables & bltcon::BIT_USED != 0 && enables & bltcon::BIT_USEC == 0 {
            // Unused entries already contain BusIdle; fill adds one such phase.
            fill[channel].1 += 1;
        }
        channel += 1;
    }
    [normal, fill]
};

/// Table 6-2. Empty phases advance locally without reserving the memory bus.
#[inline]
pub fn word_phases(bltcon0: u16, bltcon1: u16) -> ([BlitterPhase; MAX_WORD_PHASES], u8) {
    let channels = usize::from((bltcon0 & MASK_CHANNELS) >> BIT_CHANNEL_SHIFT);
    WORD_PHASES[usize::from(bltcon1 & field::MASK_FILL != 0)][channels]
}
