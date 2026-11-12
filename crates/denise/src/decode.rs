//! HRM 3 color interpretation for normal, dual, HAM6 and Extra Half-Brite pixels.

use super::Denise;
use frame_builder::{BIT_RGB444_GREEN_SHIFT, BIT_RGB444_RED_SHIFT, MASK_RGB444_COMPONENT};

const BIT_HAM_CONTROL_SHIFT: u8 = 4;
const HAM_PALETTE: u8 = 0;
const HAM_BLUE: u8 = 1;
const HAM_RED: u8 = 2;
const MASK_HAM_CONTROL: u8 = 3;
pub(super) const MASK_PALETTE_INDEX: u8 = 0x1f;
const BIT_EHB_SELECT: u8 = 0x20;
const MASK_HALF_COMPONENTS: u16 = 0x0777;
const PF2_PALETTE_BASE: usize = 8;

pub fn decode_ham6(data: u8, palette: &[u16; Denise::COLOR_PALETTE_SIZE], held: &mut u16) -> u16 {
    let component = u16::from(data) & MASK_RGB444_COMPONENT;
    let control = (data >> BIT_HAM_CONTROL_SHIFT) & MASK_HAM_CONTROL;
    if control == HAM_PALETTE {
        *held = palette[usize::from(component)];
    } else {
        let shift = match control {
            HAM_BLUE => 0,
            HAM_RED => BIT_RGB444_RED_SHIFT,
            _ => BIT_RGB444_GREEN_SHIFT,
        };
        *held = (*held & !(MASK_RGB444_COMPONENT << shift)) | (component << shift);
    }
    *held
}

pub fn decode_ehb(data: u8, palette: &[u16; Denise::COLOR_PALETTE_SIZE]) -> u16 {
    let color = palette[usize::from(data & MASK_PALETTE_INDEX)];
    if data & BIT_EHB_SELECT != 0 {
        (color >> 1) & MASK_HALF_COMPONENTS
    } else {
        color
    }
}

pub(super) fn playfield_codes(data: u8) -> (u8, u8) {
    let mut odd = 0;
    let mut even = 0;
    for bit in 0..3 {
        odd |= ((data >> (bit * 2)) & 1) << bit;
        even |= ((data >> (bit * 2 + 1)) & 1) << bit;
    }
    (odd, even)
}

pub fn decode_dual_playfield(
    data: u8,
    palette: &[u16; Denise::COLOR_PALETTE_SIZE],
    second_first: bool,
) -> u16 {
    let (first, second) = playfield_codes(data);
    let index = if second != 0 && (second_first || first == 0) {
        PF2_PALETTE_BASE + usize::from(second)
    } else {
        usize::from(first)
    };
    palette[index]
}
