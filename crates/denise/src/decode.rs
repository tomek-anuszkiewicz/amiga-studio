use super::Denise;

use frame_builder::{
    BIT_RGB444_GREEN_SHIFT, BIT_RGB444_RED_SHIFT, MASK_RGB444_COLOR, MASK_RGB444_COMPONENT,
};
const BIT_HAM_CONTROL_SHIFT: u8 = 4;
const MASK_HAM_CONTROL: u8 = 0x03;
const MASK_HAM_DATA: u8 = 0x0F;
const HAM_PALETTE: u8 = 0;
const HAM_BLUE: u8 = 1;
const HAM_RED: u8 = 2;
const HAM_GREEN: u8 = 3;
pub(super) const MASK_PALETTE_INDEX: u8 = 0x1F;
const BIT_EHB_SELECT: u8 = 0x20;
const PF2_PALETTE_BASE: usize = 8;

const BIT_PF_SECOND_PLANE_SHIFT: u8 = 1;
const BIT_PF_THIRD_PLANE_SHIFT: u8 = 2;
const MASK_PLANE_PIXEL: u8 = 1;
const BIT_PLANE1_SHIFT: u8 = 0;
const BIT_PLANE2_SHIFT: u8 = 1;
const BIT_PLANE3_SHIFT: u8 = 2;
const BIT_PLANE4_SHIFT: u8 = 3;
const BIT_PLANE5_SHIFT: u8 = 4;
const BIT_PLANE6_SHIFT: u8 = 5;

/// Decodes a HAM6 pixel given raw bitplane data and previous held RGB color
#[inline(always)]
pub fn decode_ham6(
    planes_data: u8,
    palette: &[u16; Denise::COLOR_PALETTE_SIZE],
    held_rgb: &mut u16,
) -> u16 {
    let ctrl = (planes_data >> BIT_HAM_CONTROL_SHIFT) & MASK_HAM_CONTROL;
    let data = (planes_data & MASK_HAM_DATA) as u16;

    let r = (*held_rgb >> BIT_RGB444_RED_SHIFT) & MASK_RGB444_COMPONENT;
    let g = (*held_rgb >> BIT_RGB444_GREEN_SHIFT) & MASK_RGB444_COMPONENT;
    let b = *held_rgb & MASK_RGB444_COMPONENT;

    match ctrl {
        HAM_PALETTE => {
            let col = palette[data as usize] & MASK_RGB444_COLOR;
            *held_rgb = col;
            col
        }
        HAM_BLUE => {
            let col = (r << BIT_RGB444_RED_SHIFT) | (g << BIT_RGB444_GREEN_SHIFT) | data;
            *held_rgb = col;
            col
        }
        HAM_RED => {
            let col = (data << BIT_RGB444_RED_SHIFT) | (g << BIT_RGB444_GREEN_SHIFT) | b;
            *held_rgb = col;
            col
        }
        HAM_GREEN => {
            let col = (r << BIT_RGB444_RED_SHIFT) | (data << BIT_RGB444_GREEN_SHIFT) | b;
            *held_rgb = col;
            col
        }
        _ => *held_rgb,
    }
}

/// Decodes an Extra Half-Brite (EHB) pixel given raw bitplane data
#[inline(always)]
pub fn decode_ehb(planes_data: u8, palette: &[u16; Denise::COLOR_PALETTE_SIZE]) -> u16 {
    let idx = (planes_data & MASK_PALETTE_INDEX) as usize;
    let col = palette[idx];
    if (planes_data & BIT_EHB_SELECT) != 0 {
        let r = ((col >> BIT_RGB444_RED_SHIFT) & MASK_RGB444_COMPONENT) >> 1;
        let g = ((col >> BIT_RGB444_GREEN_SHIFT) & MASK_RGB444_COMPONENT) >> 1;
        let b = (col & MASK_RGB444_COMPONENT) >> 1;
        (r << BIT_RGB444_RED_SHIFT) | (g << BIT_RGB444_GREEN_SHIFT) | b
    } else {
        col
    }
}

/// Decodes a Dual Playfield pixel given raw bitplane data and PF2 priority flag
#[inline(always)]
pub fn decode_dual_playfield(
    planes_data: u8,
    palette: &[u16; Denise::COLOR_PALETTE_SIZE],
    pf2_priority: bool,
) -> u16 {
    let pf1_idx = ((planes_data >> BIT_PLANE1_SHIFT) & MASK_PLANE_PIXEL)
        | (((planes_data >> BIT_PLANE3_SHIFT) & MASK_PLANE_PIXEL) << BIT_PF_SECOND_PLANE_SHIFT)
        | (((planes_data >> BIT_PLANE5_SHIFT) & MASK_PLANE_PIXEL) << BIT_PF_THIRD_PLANE_SHIFT);

    let pf2_idx = ((planes_data >> BIT_PLANE2_SHIFT) & MASK_PLANE_PIXEL)
        | (((planes_data >> BIT_PLANE4_SHIFT) & MASK_PLANE_PIXEL) << BIT_PF_SECOND_PLANE_SHIFT)
        | (((planes_data >> BIT_PLANE6_SHIFT) & MASK_PLANE_PIXEL) << BIT_PF_THIRD_PLANE_SHIFT);

    let pf1_col = if pf1_idx != 0 {
        Some(palette[pf1_idx as usize])
    } else {
        None
    };

    let pf2_col = if pf2_idx != 0 {
        Some(palette[PF2_PALETTE_BASE + pf2_idx as usize])
    } else {
        None
    };

    if pf2_priority {
        pf2_col.or(pf1_col).unwrap_or(palette[0])
    } else {
        pf1_col.or(pf2_col).unwrap_or(palette[0])
    }
}
