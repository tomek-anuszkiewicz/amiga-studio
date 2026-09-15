//! Blitter Execution Phases and Channel Word Sequencing
//!
//! Models HRM Table 6.2 word cycle phases across all 16 channel combinations (A-D)
//! and barrel shift arithmetic.

use serde::{Deserialize, Serialize};

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
) -> ([BlitterPhase; 4], u8) {
    let abcd = ((use_a as u8) << 3) | ((use_b as u8) << 2) | ((use_c as u8) << 1) | (use_d as u8);

    match abcd {
        // 0: -- -- (2 idle cycles per word)
        0 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            2,
        ),
        // 1: D only
        1 => {
            if fill {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                        BlitterPhase::Idle,
                    ],
                    3,
                )
            } else {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                        BlitterPhase::Idle,
                    ],
                    2,
                )
            }
        }
        // 2: C only
        2 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            2,
        ),
        // 3: C D
        3 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 4: B only
        4 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 5: B D
        5 => {
            if fill {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                    ],
                    4,
                )
            } else {
                (
                    [
                        BlitterPhase::BusIdle,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                    ],
                    3,
                )
            }
        }
        // 6: B C
        6 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 7: B C D
        7 => (
            [
                BlitterPhase::BusIdle,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
            ],
            4,
        ),
        // 8: A only
        8 => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            2,
        ),
        // 9: A D
        9 => {
            if fill {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                        BlitterPhase::Idle,
                    ],
                    3,
                )
            } else {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                        BlitterPhase::Idle,
                    ],
                    2,
                )
            }
        }
        // 10: A C
        10 => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
                BlitterPhase::Idle,
            ],
            2,
        ),
        // 11: A C D
        11 => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 12: A B
        12 => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::BusIdle,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 13: A B D
        13 => {
            if fill {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::BusIdle,
                    ],
                    4,
                )
            } else {
                (
                    [
                        BlitterPhase::FetchA,
                        BlitterPhase::FetchB,
                        BlitterPhase::WriteD,
                        BlitterPhase::Idle,
                    ],
                    3,
                )
            }
        }
        // 14: A B C
        14 => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::Idle,
            ],
            3,
        ),
        // 15: A B C D
        _ => (
            [
                BlitterPhase::FetchA,
                BlitterPhase::FetchB,
                BlitterPhase::FetchC,
                BlitterPhase::WriteD,
            ],
            4,
        ),
    }
}
