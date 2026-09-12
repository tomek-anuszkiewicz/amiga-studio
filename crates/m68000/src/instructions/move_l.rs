//! MOVE Long (32-bit) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Atomic Micro-Step Constants (2-Clock / 1 CCK)
// ============================================================================

const BUS_READ_IDLE: MicroStep = common::BUS_READ_IDLE;
const BUS_WRITE_IDLE: MicroStep = common::BUS_WRITE_IDLE;
const READ_SRC_LONG_HIGH: MicroStep = common::READ_SRC_LONG_HIGH;
const READ_SRC_LONG_LOW: MicroStep = common::READ_SRC_LONG_LOW;
const WRITE_DST_LONG_HIGH: MicroStep = common::WRITE_DST_LONG_HIGH;
const WRITE_DST_LONG_LOW: MicroStep = common::WRITE_DST_LONG_LOW;
const WRITE_DST_PD_LONG_LOW: MicroStep = common::WRITE_DST_PD_LONG_LOW;
const WRITE_DST_PD_LONG_HIGH: MicroStep = common::WRITE_DST_PD_LONG_HIGH;
const FETCH_EXT_READ: MicroStep = common::FETCH_EXT_READ;
const FETCH_EXT_FINISH: MicroStep = common::FETCH_EXT_FINISH;
const PREFETCH_NEXT_READ: MicroStep = common::PREFETCH_NEXT_READ;

// ============================================================================
// Pure ALU Callbacks: MOVE.L
// ============================================================================

pub fn alu_move_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

pub fn alu_move_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

pub fn alu_move_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

pub fn alu_move_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

pub fn alu_move_l_src_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
}

pub fn alu_move_l_src_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
}

pub fn alu_move_l_src_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
}

pub fn alu_move_l_src_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
}

pub fn alu_move_l_dn_dst_ai(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    state.micro.ea_addr = state.read_a(reg_dst as usize);
}

pub fn alu_move_l_dn_dst_pi(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    if (an & 1) == 0 {
        state.write_a(reg_dst as usize, an.wrapping_add(4));
    }
}

pub fn alu_move_l_dn_dst_pd(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize).wrapping_sub(2);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

pub fn alu_move_l_an_dst_ai(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    state.micro.ea_addr = state.read_a(reg_dst as usize);
}

pub fn alu_move_l_an_dst_pi(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    if (an & 1) == 0 {
        state.write_a(reg_dst as usize, an.wrapping_add(4));
    }
}

pub fn alu_move_l_an_dst_pd(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize).wrapping_sub(2);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

pub fn alu_move_l_mem_dst_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    state.micro.ea_addr = state.read_a(reg_dst as usize);
}

pub fn alu_move_l_mem_dst_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    if (an & 1) == 0 {
        state.write_a(reg_dst as usize, an.wrapping_add(4));
    }
}

pub fn alu_move_l_mem_dst_pd(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize).wrapping_sub(2);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

pub fn alu_move_l_imm_dst_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    state.micro.ea_addr = state.read_a(reg_dst as usize);
}

pub fn alu_move_l_imm_dst_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize);
    state.micro.ea_addr = an;
    if (an & 1) == 0 {
        state.write_a(reg_dst as usize, an.wrapping_add(4));
    }
}

pub fn alu_move_l_imm_dst_pd(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.destination = val;
    let an = state.read_a(reg_dst as usize).wrapping_sub(2);
    state.write_a(reg_dst as usize, an);
    state.micro.ea_addr = an;
}

const READ_SRC_IDLE_MOVE_L: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_move_l_src_mem),
    base_clocks: 2,
};

const FETCH_EXT_FINISH_ALU_MOVE_L_IMM: MicroStep = MicroStep {
    bus_fn: Some(Cpu::step_fetch_extension_finish),
    alu_fn: Some(alu_move_l_src_imm),
    base_clocks: 2,
};

pub fn alu_move_l_dn_ea_d16_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_dn(state, reg_src, reg_dst);
    ea::ea_calc_dst_d16_an(state, reg_src, reg_dst);
}
pub fn alu_move_l_dn_ea_idx_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_dn(state, reg_src, reg_dst);
    ea::ea_calc_dst_idx_an(state, reg_src, reg_dst);
}
pub fn alu_move_l_dn_ea_absw(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_dn(state, reg_src, reg_dst);
    ea::ea_calc_absw(state, reg_src, reg_dst);
}
pub fn alu_move_l_dn_ea_absl_hi(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_dn(state, reg_src, reg_dst);
    ea::ea_calc_absl_hi(state, reg_src, reg_dst);
}

pub fn alu_move_l_an_ea_d16_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_an(state, reg_src, reg_dst);
    ea::ea_calc_dst_d16_an(state, reg_src, reg_dst);
}
pub fn alu_move_l_an_ea_idx_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_an(state, reg_src, reg_dst);
    ea::ea_calc_dst_idx_an(state, reg_src, reg_dst);
}
pub fn alu_move_l_an_ea_absw(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_an(state, reg_src, reg_dst);
    ea::ea_calc_absw(state, reg_src, reg_dst);
}
pub fn alu_move_l_an_ea_absl_hi(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    alu_move_l_src_an(state, reg_src, reg_dst);
    ea::ea_calc_absl_hi(state, reg_src, reg_dst);
}

// ============================================================================
// Static Step Slices: MOVE.L
// ============================================================================

pub static STEPS_MOVE_L_DN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_dn_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_dn_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_dn_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_dn_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_DN_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_dn_ea_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_dn_ea_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_dn_ea_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_DN_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_dn_ea_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_an_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_an_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_an_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_an_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_AN_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_an_ea_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_an_ea_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_an_ea_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AN_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_move_l_an_ea_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_AI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_PI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_PD: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_AI_D16_AN: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_ABSW: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_AI_ABSL: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_AI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_PI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_PD: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_PI_D16_AN: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_ABSW: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PI_ABSL: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_DN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_AI: [MicroStep; 11] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_PI: [MicroStep; 11] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_PD: [MicroStep; 11] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_PD_D16_AN: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_IDX_AN: [MicroStep; 14] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_ABSW: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_PD_ABSL: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_AI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_PI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_PD: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_D16_AN_D16_AN: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_IDX_AN: [MicroStep; 15] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_ABSW: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_AN_ABSL: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_DN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_AI: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_PI: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_PD: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_IDX_AN_D16_AN: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_IDX_AN: [MicroStep; 16] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_ABSW: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_AN_ABSL: [MicroStep; 17] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_AI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_PI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_PD: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_ABSW_D16_AN: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_IDX_AN: [MicroStep; 15] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_ABSW: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSW_ABSL: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_DN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_AI: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_PI: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_PD: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_ABSL_D16_AN: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_IDX_AN: [MicroStep; 17] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_ABSW: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_ABSL_ABSL: [MicroStep; 18] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_AI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_PI: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_PD: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_D16_PC_D16_AN: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_IDX_AN: [MicroStep; 15] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_ABSW: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_D16_PC_ABSL: [MicroStep; 16] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_DN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_AI: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_PI: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_mem_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_PD: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_mem_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_IDX_PC_D16_AN: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_IDX_AN: [MicroStep; 16] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_ABSW: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IDX_PC_ABSL: [MicroStep; 17] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_LONG_HIGH,
    BUS_READ_IDLE,
    READ_SRC_LONG_LOW,
    READ_SRC_IDLE_MOVE_L,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_imm_dn),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_AI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_imm_dst_ai),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_PI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_l_imm_dst_pi),
        base_clocks: 2,
    },
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_PD: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_l_imm_dst_pd),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_LOW,
    BUS_WRITE_IDLE,
    WRITE_DST_PD_LONG_HIGH,
];
pub static STEPS_MOVE_L_IMM_D16_AN: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH_ALU_MOVE_L_IMM,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH_ALU_MOVE_L_IMM,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_ABSW: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH_ALU_MOVE_L_IMM,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];
pub static STEPS_MOVE_L_IMM_ABSL: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH_ALU_MOVE_L_IMM,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_HIGH,
    BUS_WRITE_IDLE,
    WRITE_DST_LONG_LOW,
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

// ============================================================================
// Static 2D Lookup Table: MOVE.L
// ============================================================================

static MOVE_L_LOOKUP: [[&[MicroStep]; 8]; 12] = [
    [
        &STEPS_MOVE_L_DN_DN,
        &STEPS_MOVE_L_DN_AI,
        &STEPS_MOVE_L_DN_PI,
        &STEPS_MOVE_L_DN_PD,
        &STEPS_MOVE_L_DN_D16_AN,
        &STEPS_MOVE_L_DN_IDX_AN,
        &STEPS_MOVE_L_DN_ABSW,
        &STEPS_MOVE_L_DN_ABSL,
    ],
    [
        &STEPS_MOVE_L_AN_DN,
        &STEPS_MOVE_L_AN_AI,
        &STEPS_MOVE_L_AN_PI,
        &STEPS_MOVE_L_AN_PD,
        &STEPS_MOVE_L_AN_D16_AN,
        &STEPS_MOVE_L_AN_IDX_AN,
        &STEPS_MOVE_L_AN_ABSW,
        &STEPS_MOVE_L_AN_ABSL,
    ],
    [
        &STEPS_MOVE_L_AI_DN,
        &STEPS_MOVE_L_AI_AI,
        &STEPS_MOVE_L_AI_PI,
        &STEPS_MOVE_L_AI_PD,
        &STEPS_MOVE_L_AI_D16_AN,
        &STEPS_MOVE_L_AI_IDX_AN,
        &STEPS_MOVE_L_AI_ABSW,
        &STEPS_MOVE_L_AI_ABSL,
    ],
    [
        &STEPS_MOVE_L_PI_DN,
        &STEPS_MOVE_L_PI_AI,
        &STEPS_MOVE_L_PI_PI,
        &STEPS_MOVE_L_PI_PD,
        &STEPS_MOVE_L_PI_D16_AN,
        &STEPS_MOVE_L_PI_IDX_AN,
        &STEPS_MOVE_L_PI_ABSW,
        &STEPS_MOVE_L_PI_ABSL,
    ],
    [
        &STEPS_MOVE_L_PD_DN,
        &STEPS_MOVE_L_PD_AI,
        &STEPS_MOVE_L_PD_PI,
        &STEPS_MOVE_L_PD_PD,
        &STEPS_MOVE_L_PD_D16_AN,
        &STEPS_MOVE_L_PD_IDX_AN,
        &STEPS_MOVE_L_PD_ABSW,
        &STEPS_MOVE_L_PD_ABSL,
    ],
    [
        &STEPS_MOVE_L_D16_AN_DN,
        &STEPS_MOVE_L_D16_AN_AI,
        &STEPS_MOVE_L_D16_AN_PI,
        &STEPS_MOVE_L_D16_AN_PD,
        &STEPS_MOVE_L_D16_AN_D16_AN,
        &STEPS_MOVE_L_D16_AN_IDX_AN,
        &STEPS_MOVE_L_D16_AN_ABSW,
        &STEPS_MOVE_L_D16_AN_ABSL,
    ],
    [
        &STEPS_MOVE_L_IDX_AN_DN,
        &STEPS_MOVE_L_IDX_AN_AI,
        &STEPS_MOVE_L_IDX_AN_PI,
        &STEPS_MOVE_L_IDX_AN_PD,
        &STEPS_MOVE_L_IDX_AN_D16_AN,
        &STEPS_MOVE_L_IDX_AN_IDX_AN,
        &STEPS_MOVE_L_IDX_AN_ABSW,
        &STEPS_MOVE_L_IDX_AN_ABSL,
    ],
    [
        &STEPS_MOVE_L_ABSW_DN,
        &STEPS_MOVE_L_ABSW_AI,
        &STEPS_MOVE_L_ABSW_PI,
        &STEPS_MOVE_L_ABSW_PD,
        &STEPS_MOVE_L_ABSW_D16_AN,
        &STEPS_MOVE_L_ABSW_IDX_AN,
        &STEPS_MOVE_L_ABSW_ABSW,
        &STEPS_MOVE_L_ABSW_ABSL,
    ],
    [
        &STEPS_MOVE_L_ABSL_DN,
        &STEPS_MOVE_L_ABSL_AI,
        &STEPS_MOVE_L_ABSL_PI,
        &STEPS_MOVE_L_ABSL_PD,
        &STEPS_MOVE_L_ABSL_D16_AN,
        &STEPS_MOVE_L_ABSL_IDX_AN,
        &STEPS_MOVE_L_ABSL_ABSW,
        &STEPS_MOVE_L_ABSL_ABSL,
    ],
    [
        &STEPS_MOVE_L_D16_PC_DN,
        &STEPS_MOVE_L_D16_PC_AI,
        &STEPS_MOVE_L_D16_PC_PI,
        &STEPS_MOVE_L_D16_PC_PD,
        &STEPS_MOVE_L_D16_PC_D16_AN,
        &STEPS_MOVE_L_D16_PC_IDX_AN,
        &STEPS_MOVE_L_D16_PC_ABSW,
        &STEPS_MOVE_L_D16_PC_ABSL,
    ],
    [
        &STEPS_MOVE_L_IDX_PC_DN,
        &STEPS_MOVE_L_IDX_PC_AI,
        &STEPS_MOVE_L_IDX_PC_PI,
        &STEPS_MOVE_L_IDX_PC_PD,
        &STEPS_MOVE_L_IDX_PC_D16_AN,
        &STEPS_MOVE_L_IDX_PC_IDX_AN,
        &STEPS_MOVE_L_IDX_PC_ABSW,
        &STEPS_MOVE_L_IDX_PC_ABSL,
    ],
    [
        &STEPS_MOVE_L_IMM_DN,
        &STEPS_MOVE_L_IMM_AI,
        &STEPS_MOVE_L_IMM_PI,
        &STEPS_MOVE_L_IMM_PD,
        &STEPS_MOVE_L_IMM_D16_AN,
        &STEPS_MOVE_L_IMM_IDX_AN,
        &STEPS_MOVE_L_IMM_ABSW,
        &STEPS_MOVE_L_IMM_ABSL,
    ],
];

// ============================================================================
// Opcode Decoder: MOVE.L
// ============================================================================

pub const fn decode_move_l_steps(
    src_mode: u8,
    src_reg: u8,
    dst_mode: u8,
    dst_reg: u8,
) -> Option<&'static [MicroStep]> {
    let src_idx = match src_mode {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => match src_reg {
            0 => 7,
            1 => 8,
            2 => 9,
            3 => 10,
            4 => 11,
            _ => return None,
        },
        _ => return None,
    };
    let dst_idx = match dst_mode {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => match dst_reg {
            0 => 6,
            1 => 7,
            _ => return None,
        },
        _ => return None,
    };
    Some(MOVE_L_LOOKUP[src_idx][dst_idx])
}
