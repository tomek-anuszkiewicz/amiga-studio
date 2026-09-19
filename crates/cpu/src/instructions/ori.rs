//! M68000 ORI Instruction (`ORI #<data>, <ea>`)
//!
//! Performs bitwise OR of immediate data with a destination data register or alterable memory location.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

pub fn alu_ori_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = crate::instructions::or::or_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_ori_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = crate::instructions::or::or_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_ori_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    let res = crate::instructions::or::or_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
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

pub fn alu_ori_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = crate::instructions::or::or_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

pub fn alu_ori_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = crate::instructions::or::or_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

pub fn alu_ori_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    let res = crate::instructions::or::or_l(state, s, d);
    state.micro.destination = res;
}

// ============================================================================
// Static Micro-Step Slices: ORI Byte
// ============================================================================

pub static STEPS_ORI_B_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_ori_b_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_ORI_B_AI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_PI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_D16_AN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_IDX_AN: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_ABSW: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_ORI_B_ABSL: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_b),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: ORI Word
// ============================================================================

pub static STEPS_ORI_W_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_ori_w_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_ORI_W_AI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_PI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_D16_AN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_IDX_AN: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_ABSW: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_ORI_W_ABSL: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Micro-Step Slices: ORI Long
// ============================================================================

pub static STEPS_ORI_L_DN: [MicroStep; 7] = [
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
    common::ALU_IDLE_4CLK,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_imm_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ORI_L_AI: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_PI: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_PD: [MicroStep; 15] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_D16_AN: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_IDX_AN: [MicroStep; 17] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_ABSW: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_ORI_L_ABSL: [MicroStep; 18] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ori_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

// ============================================================================
// Decoder: ORI #<imm>, <ea>
// ============================================================================

pub const fn decode_ori_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_ORI_B_DN),
            2 => Some(&STEPS_ORI_B_AI),
            3 => Some(&STEPS_ORI_B_PI),
            4 => Some(&STEPS_ORI_B_PD),
            5 => Some(&STEPS_ORI_B_D16_AN),
            6 => Some(&STEPS_ORI_B_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_ORI_B_ABSW),
                1 => Some(&STEPS_ORI_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_ORI_W_DN),
            2 => Some(&STEPS_ORI_W_AI),
            3 => Some(&STEPS_ORI_W_PI),
            4 => Some(&STEPS_ORI_W_PD),
            5 => Some(&STEPS_ORI_W_D16_AN),
            6 => Some(&STEPS_ORI_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_ORI_W_ABSW),
                1 => Some(&STEPS_ORI_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_ORI_L_DN),
            2 => Some(&STEPS_ORI_L_AI),
            3 => Some(&STEPS_ORI_L_PI),
            4 => Some(&STEPS_ORI_L_PD),
            5 => Some(&STEPS_ORI_L_D16_AN),
            6 => Some(&STEPS_ORI_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_ORI_L_ABSW),
                1 => Some(&STEPS_ORI_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
