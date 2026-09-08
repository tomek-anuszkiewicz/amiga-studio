//! MOVEA (Move Address) instruction handlers
//!
//! Loads an effective address operand into an Address Register (An)
//! with word sign-extension, leaving CCR completely untouched.

use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Pure ALU Callbacks: MOVEA Word
// ============================================================================

#[inline(always)]
pub fn alu_movea_w_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.d_word(reg_src as usize) as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.a_word(reg_src as usize) as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.micro.last_read as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_imm(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.prefetch[0] as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

// ============================================================================
// Pure ALU Callbacks: MOVEA Long
// ============================================================================

#[inline(always)]
pub fn alu_movea_l_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_l_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_l_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.scratch[1];
    state.write_a(reg_dst as usize, val);
}

// ============================================================================
// Static Step Slices: MOVEA Word
// ============================================================================

pub static STEPS_MOVEA_W_DN: [MicroStep; 1] = [MicroStep {
    action: MicroAction::PrefetchNextOpcodeAndRetire,
    alu_fn: Some(alu_movea_w_dn),
    base_clocks: 4,
}];

pub static STEPS_MOVEA_W_AN: [MicroStep; 1] = [MicroStep {
    action: MicroAction::PrefetchNextOpcodeAndRetire,
    alu_fn: Some(alu_movea_w_an),
    base_clocks: 4,
}];

pub static STEPS_MOVEA_W_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_D16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_IDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_ABSW: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_ABSL: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_PCD16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_PCIDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_w_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_W_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_movea_w_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4 },
];

// ============================================================================
// Static Step Slices: MOVEA Long
// ============================================================================

pub static STEPS_MOVEA_L_DN: [MicroStep; 1] = [MicroStep {
    action: MicroAction::PrefetchNextOpcodeAndRetire,
    alu_fn: Some(alu_movea_l_dn),
    base_clocks: 4,
}];

pub static STEPS_MOVEA_L_AN: [MicroStep; 1] = [MicroStep {
    action: MicroAction::PrefetchNextOpcodeAndRetire,
    alu_fn: Some(alu_movea_l_an),
    base_clocks: 4,
}];

pub static STEPS_MOVEA_L_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_PCD16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_PCIDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

pub static STEPS_MOVEA_L_IMM: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_movea_l_mem), base_clocks: 4 },
];

/// Compile-time opcode decoder for MOVEA
pub const fn decode_movea_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if is_long {
        match mode {
            0 => Some(&STEPS_MOVEA_L_DN),
            1 => Some(&STEPS_MOVEA_L_AN),
            2 => Some(&STEPS_MOVEA_L_AI),
            3 => Some(&STEPS_MOVEA_L_PI),
            4 => Some(&STEPS_MOVEA_L_PD),
            5 => Some(&STEPS_MOVEA_L_D16),
            6 => Some(&STEPS_MOVEA_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEA_L_ABSW),
                1 => Some(&STEPS_MOVEA_L_ABSL),
                2 => Some(&STEPS_MOVEA_L_PCD16),
                3 => Some(&STEPS_MOVEA_L_PCIDX),
                4 => Some(&STEPS_MOVEA_L_IMM),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            0 => Some(&STEPS_MOVEA_W_DN),
            1 => Some(&STEPS_MOVEA_W_AN),
            2 => Some(&STEPS_MOVEA_W_AI),
            3 => Some(&STEPS_MOVEA_W_PI),
            4 => Some(&STEPS_MOVEA_W_PD),
            5 => Some(&STEPS_MOVEA_W_D16),
            6 => Some(&STEPS_MOVEA_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEA_W_ABSW),
                1 => Some(&STEPS_MOVEA_W_ABSL),
                2 => Some(&STEPS_MOVEA_W_PCD16),
                3 => Some(&STEPS_MOVEA_W_PCIDX),
                4 => Some(&STEPS_MOVEA_W_IMM),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Sign-extends a word to 32 bits for MOVEA.W
#[inline(always)]
pub fn sign_extend_word(val: u16) -> u32 {
    (val as i16 as i32) as u32
}


