//! HRM chapter 6 bus sequences and cross-word shifting.

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

/// Table 6-2. Empty phases advance locally without reserving the memory bus.
pub fn word_phases(
    use_a: bool,
    use_b: bool,
    use_c: bool,
    use_d: bool,
    fill: bool,
) -> ([BlitterPhase; MAX_WORD_PHASES], u8) {
    use BlitterPhase::{BusIdle as I, FetchA as A, FetchB as B, FetchC as C, WriteD as D};
    let (mut phases, mut count) = match (use_a, use_b, use_c, use_d) {
        (false, false, false, false) => ([I, I, I, I], 2),
        (false, false, false, true) => ([I, D, I, I], 2),
        (false, false, true, false) => ([I, C, I, I], 2),
        (false, false, true, true) => ([I, C, D, I], 3),
        (false, true, false, false) => ([I, B, I, I], 3),
        (false, true, false, true) => ([I, B, D, I], 3),
        (false, true, true, false) => ([I, B, C, I], 3),
        (false, true, true, true) => ([I, B, C, D], 4),
        (true, false, false, false) => ([A, I, I, I], 2),
        (true, false, false, true) => ([A, D, I, I], 2),
        (true, false, true, false) => ([A, C, I, I], 2),
        (true, false, true, true) => ([A, C, D, I], 3),
        (true, true, false, false) => ([A, B, I, I], 3),
        (true, true, false, true) => ([A, B, D, I], 3),
        (true, true, true, false) => ([A, B, C, I], 3),
        (true, true, true, true) => ([A, B, C, D], 4),
    };
    if fill && use_d && !use_c {
        phases[count as usize] = I;
        count += 1;
    }
    (phases, count)
}
