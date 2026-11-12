//! MOVEA (Move Address) instruction handlers
//!
//! Loads an effective address operand into an Address Register (An)
//! with word sign-extension, leaving CCR completely untouched.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Pure ALU Callbacks: MOVEA Word
// ============================================================================

pub fn alu_movea_w_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.d_word(reg_src as usize) as i16 as i32) as u32;
    state.set_a_long(reg_dst as usize, val);
}

pub fn alu_movea_w_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.a_word(reg_src as usize) as i16 as i32) as u32;
    state.set_a_long(reg_dst as usize, val);
}

pub fn alu_movea_w_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.micro().source as i16 as i32) as u32;
    state.set_a_long(reg_dst as usize, val);
}

pub fn alu_movea_w_imm(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.prefetch() as i16 as i32) as u32;
    state.set_a_long(reg_dst as usize, val);
}

// ============================================================================
// Pure ALU Callbacks: MOVEA Long
// ============================================================================

pub fn alu_movea_l_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_a_long(reg_dst as usize, val);
}

pub fn alu_movea_l_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.a_long(reg_src as usize);
    state.set_a_long(reg_dst as usize, val);
}

pub fn alu_movea_l_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro().source;
    state.set_a_long(reg_dst as usize, val);
}

// ============================================================================
// Static Step Slices: MOVEA Word
// ============================================================================

pub static STEPS_MOVEA_W_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_AN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_PI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_PD: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_D16_AN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_IDX_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_ABSW: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_ABSL: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_D16_PC: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_IDX_PC: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_W_IMM: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_movea_w_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Step Slices: MOVEA Long
// ============================================================================

pub static STEPS_MOVEA_L_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_AN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_D16_PC: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_IDX_PC: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEA_L_IMM: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for MOVEA
// ============================================================================

/// Compile-time opcode decoder for MOVEA
pub const fn decode_movea_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if is_long {
        match mode {
            mode::DATA_REGISTER => Some(&STEPS_MOVEA_L_DN),
            mode::ADDRESS_REGISTER => Some(&STEPS_MOVEA_L_AN),
            mode::INDIRECT => Some(&STEPS_MOVEA_L_AI),
            mode::POSTINCREMENT => Some(&STEPS_MOVEA_L_PI),
            mode::PREDECREMENT => Some(&STEPS_MOVEA_L_PD),
            mode::DISPLACEMENT => Some(&STEPS_MOVEA_L_D16_AN),
            mode::INDEXED => Some(&STEPS_MOVEA_L_IDX_AN),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_MOVEA_L_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_MOVEA_L_ABSL),
                extended_mode::PC_DISPLACEMENT => Some(&STEPS_MOVEA_L_D16_PC),
                extended_mode::PC_INDEXED => Some(&STEPS_MOVEA_L_IDX_PC),
                extended_mode::IMMEDIATE => Some(&STEPS_MOVEA_L_IMM),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            mode::DATA_REGISTER => Some(&STEPS_MOVEA_W_DN),
            mode::ADDRESS_REGISTER => Some(&STEPS_MOVEA_W_AN),
            mode::INDIRECT => Some(&STEPS_MOVEA_W_AI),
            mode::POSTINCREMENT => Some(&STEPS_MOVEA_W_PI),
            mode::PREDECREMENT => Some(&STEPS_MOVEA_W_PD),
            mode::DISPLACEMENT => Some(&STEPS_MOVEA_W_D16_AN),
            mode::INDEXED => Some(&STEPS_MOVEA_W_IDX_AN),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_MOVEA_W_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_MOVEA_W_ABSL),
                extended_mode::PC_DISPLACEMENT => Some(&STEPS_MOVEA_W_D16_PC),
                extended_mode::PC_INDEXED => Some(&STEPS_MOVEA_W_IDX_PC),
                extended_mode::IMMEDIATE => Some(&STEPS_MOVEA_W_IMM),
                _ => None,
            },
            _ => None,
        }
    }
}
