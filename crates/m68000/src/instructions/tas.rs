//! TAS (Test and Set an Operand) Instruction Handler
//!
//! Tests a destination byte, sets Condition Codes (N, Z, V=0, C=0, X unaffected),
//! and sets bit 7 (MSB) of the byte (`byte | 0x80`).
//!
//! Valid EA modes: Data alterable (Dn, (An), (An)+, -(An), (d16,An), (d8,An,Xn), (xxx).W, (xxx).L).
//! Execution time: 4 clocks (Dn), 14..22 clocks (memory).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_tas_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let orig = state.d_long(reg_dst as usize);
    let val = (orig & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val & 0x80) != 0, val == 0);
    let new_byte = val | 0x80;
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (new_byte as u32));
}

pub static ALU_TAS_DN: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_tas_dn),
    base_clocks: 0,
};

pub fn alu_tas_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.micro.source & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val & 0x80) != 0, val == 0);
    state.micro.destination = (val | 0x80) as u32;
}

pub static ALU_TAS_MEM_2CLK: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_tas_mem),
    base_clocks: 2,
};

// ============================================================================
// Static Micro-Step Slices for TAS
// ============================================================================

pub static STEPS_TAS_DN: [MicroStep; 3] = [
    ALU_TAS_DN,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AI: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_PI: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_PD: [MicroStep; 8] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_D16: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_IDX: [MicroStep; 10] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AW: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_TAS_AL: [MicroStep; 11] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    ALU_TAS_MEM_2CLK,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Opcode Decoding Helper
// ============================================================================

pub const fn decode_tas_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_TAS_DN),
        2 => Some(&STEPS_TAS_AI),
        3 => Some(&STEPS_TAS_PI),
        4 => Some(&STEPS_TAS_PD),
        5 => Some(&STEPS_TAS_D16),
        6 => Some(&STEPS_TAS_IDX),
        7 => match reg {
            0 => Some(&STEPS_TAS_AW),
            1 => Some(&STEPS_TAS_AL),
            _ => None,
        },
        _ => None,
    }
}
