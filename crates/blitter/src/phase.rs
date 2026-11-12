//! Blitter Execution Phases and Channel Word Sequencing
//!
//! Models HRM Table 6.2 word cycle phases across all 16 channel combinations (A-D)
//! and barrel shift arithmetic.

use serde::{Deserialize, Serialize};

const WORD_CYCLES_NONE: u8 = 2;
const WORD_CYCLES_D_FILL: u8 = 3;
const WORD_CYCLES_D: u8 = 2;
const WORD_CYCLES_C: u8 = 2;
const WORD_CYCLES_CD: u8 = 3;
const WORD_CYCLES_B: u8 = 3;
const WORD_CYCLES_BD_FILL: u8 = 4;
const WORD_CYCLES_BD: u8 = 3;
const WORD_CYCLES_BC: u8 = 3;
const WORD_CYCLES_BCD: u8 = 4;
const WORD_CYCLES_A: u8 = 2;
const WORD_CYCLES_AD_FILL: u8 = 3;
const WORD_CYCLES_AD: u8 = 2;
const WORD_CYCLES_AC: u8 = 2;
const WORD_CYCLES_ACD: u8 = 3;
const WORD_CYCLES_AB: u8 = 3;
const WORD_CYCLES_A_B_D_FILL: u8 = 4;
const WORD_CYCLES_A_B_D: u8 = 3;
const WORD_CYCLES_A_B_C: u8 = 3;
const WORD_CYCLES_ABCD: u8 = 4;

pub const MAX_WORD_PHASES: usize = 4;
const CHANNELS_NONE: u8 = 0;
const CHANNELS_D: u8 = 1;
const CHANNELS_C: u8 = 2;
const CHANNELS_CD: u8 = 3;
const CHANNELS_B: u8 = 4;
const CHANNELS_BD: u8 = 5;
const CHANNELS_BC: u8 = 6;
const CHANNELS_BCD: u8 = 7;
const CHANNELS_A: u8 = 8;
const CHANNELS_AD: u8 = 9;
const CHANNELS_AC: u8 = 10;
const CHANNELS_ACD: u8 = 11;
const CHANNELS_AB: u8 = 12;
const CHANNELS_A_B_D: u8 = 13;
const CHANNELS_A_B_C: u8 = 14;

/// Active execution phase for cycle-by-cycle Blitter stepping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlitterPhase {
    /// Blitter is idle
    #[default]
    Idle,
    /// Channel A DMA fetch
    FetchA,
    /// Channel B DMA fetch
    FetchB,
    /// Channel C DMA fetch
    FetchC,
    /// Destination D computation and write
    WriteD,
    /// Internal idle cycle (Table 6.2)
    BusIdle,
    /// Line mode single pixel step
    LinePixel,
}

/// Computes barrel-shifted data for 16-bit words across word boundaries
#[inline(always)]
pub fn barrel_shift(anew: u16, aold: u16, shift: u16, desc: bool) -> u16 {
    if desc {
        if shift == 0 {
            anew
        } else {
            ((((anew as u32) << 16) | (aold as u32)) >> (16 - shift)) as u16
        }
    } else {
        ((((aold as u32) << 16) | (anew as u32)) >> shift) as u16
    }
}

/// Returns the active micro-phase sequence for a word in Area Mode according to HRM Table 6.2
#[inline]
pub fn word_phases(
    use_a: bool,
    use_b: bool,
    use_c: bool,
    use_d: bool,
    fill: bool,
) -> ([BlitterPhase; MAX_WORD_PHASES], u8) {
    let abcd = ((use_a as u8) * CHANNELS_A)
        | ((use_b as u8) * CHANNELS_B)
        | ((use_c as u8) * CHANNELS_C)
        | ((use_d as u8) * CHANNELS_D);

    match abcd {
        // 0: -- -- (2 idle cycles per word)
        CHANNELS_NONE => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_NONE,
        ),
        // 1: D only
        CHANNELS_D => {
            if fill {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_D_FILL,
                )
            } else {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_D,
                )
            }
        }
        // 2: C only
        CHANNELS_C => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_C,
        ),
        // 3: C D
        CHANNELS_CD => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_CD,
        ),
        // 4: B only
        CHANNELS_B => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_B,
        ),
        // 5: B D
        CHANNELS_BD => {
            if fill {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                    ],
                    WORD_CYCLES_BD_FILL,
                )
            } else {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_BD,
                )
            }
        }
        // 6: B C
        CHANNELS_BC => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_BC,
        ),
        // 7: B C D
        CHANNELS_BCD => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
            ],
            WORD_CYCLES_BCD,
        ),
        // 8: A only
        CHANNELS_A => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_A,
        ),
        // 9: A D
        CHANNELS_AD => {
            if fill {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_AD_FILL,
                )
            } else {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_AD,
                )
            }
        }
        // 10: A C
        CHANNELS_AC => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_AC,
        ),
        // 11: A C D
        CHANNELS_ACD => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_ACD,
        ),
        // 12: A B
        CHANNELS_AB => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_AB,
        ),
        // 13: A B D
        CHANNELS_A_B_D => {
            if fill {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                    ],
                    WORD_CYCLES_A_B_D_FILL,
                )
            } else {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                    ],
                    WORD_CYCLES_A_B_D,
                )
            }
        }
        // 14: A B C
        CHANNELS_A_B_C => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
            ],
            WORD_CYCLES_A_B_C,
        ),
        // 15: A B C D
        _ => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
            ],
            WORD_CYCLES_ABCD,
        ),
    }
}
