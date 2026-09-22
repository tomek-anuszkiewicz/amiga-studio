//! M68000 CMPA Instruction (`CMPA <ea>, An`)
//!
//! Compares effective address operand with address register: `An - <ea>`.
//! Sign-extends word operands to 32 bits prior to comparison.
//! Evaluates condition codes (N, Z, V, C) while leaving X and register contents unchanged.

use crate::instructions::cmpm::cmp_l;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_cmpa_w_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) as i16 as i32) as u32;
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.a_long(reg_src as usize) as i16 as i32) as u32;
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.a_long(reg_src as usize);
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source as i16 as i32) as u32;
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_imm_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch as i16 as i32) as u32;
    let d = state.a_long(reg_dst as usize);
    cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices: CMPA.W <ea>, An
// ============================================================================

pub static STEPS_CMPA_W_DN_AN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_dn_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPA_W_AN_AN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_an_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPA_W_AI_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_PI_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_PD_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_D16_AN_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_IDX_AN_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_ABSW_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_ABSL_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_D16_PC_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_IDX_PC_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_w_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_W_IMM_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_cmpa_w_imm_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: CMPA.L <ea>, An
// ============================================================================

pub static STEPS_CMPA_L_DN_AN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_dn_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPA_L_AN_AN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_an_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPA_L_AI_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_PI_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_PD_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_D16_AN_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_IDX_AN_AN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_ABSW_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_ABSL_AN: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_D16_PC_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_IDX_PC_AN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_CMPA_L_IMM_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpa_l_mem_an),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for CMPA
// ============================================================================

pub const fn decode_cmpa_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if !is_long {
        // CMPA.W
        match mode {
            0 => Some(&STEPS_CMPA_W_DN_AN),
            1 => Some(&STEPS_CMPA_W_AN_AN),
            2 => Some(&STEPS_CMPA_W_AI_AN),
            3 => Some(&STEPS_CMPA_W_PI_AN),
            4 => Some(&STEPS_CMPA_W_PD_AN),
            5 => Some(&STEPS_CMPA_W_D16_AN_AN),
            6 => Some(&STEPS_CMPA_W_IDX_AN_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPA_W_ABSW_AN),
                1 => Some(&STEPS_CMPA_W_ABSL_AN),
                2 => Some(&STEPS_CMPA_W_D16_PC_AN),
                3 => Some(&STEPS_CMPA_W_IDX_PC_AN),
                4 => Some(&STEPS_CMPA_W_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    } else {
        // CMPA.L
        match mode {
            0 => Some(&STEPS_CMPA_L_DN_AN),
            1 => Some(&STEPS_CMPA_L_AN_AN),
            2 => Some(&STEPS_CMPA_L_AI_AN),
            3 => Some(&STEPS_CMPA_L_PI_AN),
            4 => Some(&STEPS_CMPA_L_PD_AN),
            5 => Some(&STEPS_CMPA_L_D16_AN_AN),
            6 => Some(&STEPS_CMPA_L_IDX_AN_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPA_L_ABSW_AN),
                1 => Some(&STEPS_CMPA_L_ABSL_AN),
                2 => Some(&STEPS_CMPA_L_D16_PC_AN),
                3 => Some(&STEPS_CMPA_L_IDX_PC_AN),
                4 => Some(&STEPS_CMPA_L_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    }
}
