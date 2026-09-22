//! Pixel decoding routines for Denise display modes: HAM6, EHB, and Dual Playfield.

use super::COLOR_PALETTE_SIZE;

/// Decodes a HAM6 pixel given raw bitplane data and previous held RGB color
#[inline(always)]
pub fn decode_ham6(
    planes_data: u8,
    palette: &[u16; COLOR_PALETTE_SIZE],
    held_rgb: &mut u16,
) -> u16 {
    let ctrl = (planes_data >> 4) & 0x03;
    let data = (planes_data & 0x0F) as u16;

    let r = (*held_rgb >> 8) & 0xF;
    let g = (*held_rgb >> 4) & 0xF;
    let b = *held_rgb & 0xF;

    match ctrl {
        0 => {
            let col = palette[data as usize] & 0x0FFF;
            *held_rgb = col;
            col
        }
        1 => {
            let col = (r << 8) | (g << 4) | data;
            *held_rgb = col;
            col
        }
        2 => {
            let col = (data << 8) | (g << 4) | b;
            *held_rgb = col;
            col
        }
        3 => {
            let col = (r << 8) | (data << 4) | b;
            *held_rgb = col;
            col
        }
        _ => *held_rgb,
    }
}

/// Decodes an Extra Half-Brite (EHB) pixel given raw bitplane data
#[inline(always)]
pub fn decode_ehb(planes_data: u8, palette: &[u16; COLOR_PALETTE_SIZE]) -> u16 {
    let idx = (planes_data & 0x1F) as usize;
    let col = palette[idx];
    if (planes_data & 0x20) != 0 {
        let r = ((col >> 8) & 0xF) >> 1;
        let g = ((col >> 4) & 0xF) >> 1;
        let b = (col & 0xF) >> 1;
        (r << 8) | (g << 4) | b
    } else {
        col
    }
}

/// Decodes a Dual Playfield pixel given raw bitplane data and PF2 priority flag
#[inline(always)]
pub fn decode_dual_playfield(
    planes_data: u8,
    palette: &[u16; COLOR_PALETTE_SIZE],
    pf2_priority: bool,
) -> u16 {
    let pf1_idx = (planes_data & 0x01)
        | (((planes_data >> 2) & 0x01) << 1)
        | (((planes_data >> 4) & 0x01) << 2);

    let pf2_idx = ((planes_data >> 1) & 0x01)
        | (((planes_data >> 3) & 0x01) << 1)
        | (((planes_data >> 5) & 0x01) << 2);

    let pf1_col = if pf1_idx != 0 {
        Some(palette[pf1_idx as usize])
    } else {
        None
    };

    let pf2_col = if pf2_idx != 0 {
        Some(palette[8 + pf2_idx as usize])
    } else {
        None
    };

    if pf2_priority {
        pf2_col.or(pf1_col).unwrap_or(palette[0])
    } else {
        pf1_col.or(pf2_col).unwrap_or(palette[0])
    }
}
