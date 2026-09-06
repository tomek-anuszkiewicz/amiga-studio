//! M68000 Bit Manipulation Instructions (BTST, BSET, BCLR, BCHG)

use crate::state::CpuState;

pub fn execute_btst(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) {
    let bit_idx = if is_register { bit_num % 32 } else { bit_num % 8 };
    let bit_val = (val & (1 << bit_idx)) != 0;
    // Z is set if bit was zero; cleared if bit was one
    state.set_z(!bit_val);
}

pub fn execute_bset(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) -> u32 {
    let bit_idx = if is_register { bit_num % 32 } else { bit_num % 8 };
    let bit_mask = 1 << bit_idx;
    let bit_val = (val & bit_mask) != 0;
    state.set_z(!bit_val);
    val | bit_mask
}

pub fn execute_bclr(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) -> u32 {
    let bit_idx = if is_register { bit_num % 32 } else { bit_num % 8 };
    let bit_mask = 1 << bit_idx;
    let bit_val = (val & bit_mask) != 0;
    state.set_z(!bit_val);
    val & !bit_mask
}

pub fn execute_bchg(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) -> u32 {
    let bit_idx = if is_register { bit_num % 32 } else { bit_num % 8 };
    let bit_mask = 1 << bit_idx;
    let bit_val = (val & bit_mask) != 0;
    state.set_z(!bit_val);
    val ^ bit_mask
}
