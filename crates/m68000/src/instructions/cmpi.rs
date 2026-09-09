//! M68000 CMPI Instruction (`CMPI #<imm>, <ea>`)
//!
//! Compares immediate operand with effective address operand: `<ea> - #<imm>`.
//! Condition codes (N, Z, V, C) are set according to the result.
//! Neither operand is altered. Extend (X) flag is unaffected.

use crate::core::Cpu;
use crate::instructions::cmpm::{cmp_b, cmp_l, cmp_w};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_cmpi_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmpi_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmpi_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn latch_imm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
}

pub fn latch_imm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
}

pub fn latch_imm_b_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_b_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
    ea::ea_calc_dst_pi_w(state, 0, reg_dst);
}

pub fn latch_imm_l_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = (state.prefetch[0] as u32) << 16;
}

pub fn latch_imm_l_lo_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_l_lo_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
    ea::ea_calc_dst_pi_l(state, 0, reg_dst);
}

pub fn latch_imm_l_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
}

pub fn alu_cmpi_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmpi_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmpi_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices: CMPI Byte
// ============================================================================

pub static STEPS_CMPI_B_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_cmpi_b_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_PD: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_ABSL: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_PC_D16: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_B_PC_IDX: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: CMPI Word
// ============================================================================

pub static STEPS_CMPI_W_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_cmpi_w_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_PD: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_ABSL: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_PC_D16: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_W_PC_IDX: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
// Static Micro-Step Slices: CMPI Long
// ============================================================================

pub static STEPS_CMPI_L_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_cmpi_l_imm_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_AI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_PI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_PD: [MicroStep; 11] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_D16_AN: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_ABSW: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_ABSL: [MicroStep; 14] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_PC_D16: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPI_L_PC_IDX: [MicroStep; 13] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpi_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
// Instruction Decoder Callback
// ============================================================================

pub const fn decode_cmpi_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_CMPI_B_DN),
            2 => Some(&STEPS_CMPI_B_AI),
            3 => Some(&STEPS_CMPI_B_PI),
            4 => Some(&STEPS_CMPI_B_PD),
            5 => Some(&STEPS_CMPI_B_D16_AN),
            6 => Some(&STEPS_CMPI_B_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_B_ABSW),
                1 => Some(&STEPS_CMPI_B_ABSL),
                2 => Some(&STEPS_CMPI_B_PC_D16),
                3 => Some(&STEPS_CMPI_B_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_CMPI_W_DN),
            2 => Some(&STEPS_CMPI_W_AI),
            3 => Some(&STEPS_CMPI_W_PI),
            4 => Some(&STEPS_CMPI_W_PD),
            5 => Some(&STEPS_CMPI_W_D16_AN),
            6 => Some(&STEPS_CMPI_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_W_ABSW),
                1 => Some(&STEPS_CMPI_W_ABSL),
                2 => Some(&STEPS_CMPI_W_PC_D16),
                3 => Some(&STEPS_CMPI_W_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_CMPI_L_DN),
            2 => Some(&STEPS_CMPI_L_AI),
            3 => Some(&STEPS_CMPI_L_PI),
            4 => Some(&STEPS_CMPI_L_PD),
            5 => Some(&STEPS_CMPI_L_D16_AN),
            6 => Some(&STEPS_CMPI_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_L_ABSW),
                1 => Some(&STEPS_CMPI_L_ABSL),
                2 => Some(&STEPS_CMPI_L_PC_D16),
                3 => Some(&STEPS_CMPI_L_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
