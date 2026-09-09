//! M68000 BCD Arithmetic Instructions (`ABCD`, `SBCD`, `NBCD`)
//!
//! Binary-Coded Decimal arithmetic with extend (X) flag:
//! - `ABCD Dy, Dx` / `ABCD -(Ay), -(Ax)`: `Source_BCD + Destination_BCD + X -> Destination_BCD`
//! - `SBCD Dy, Dx` / `SBCD -(Ay), -(Ax)`: `Destination_BCD - Source_BCD - X -> Destination_BCD`
//! - `NBCD <ea>`: `0 - Destination_BCD - X -> Destination_BCD`
//!
//! Flags:
//! - X = C (decimal carry/borrow)
//! - N = MSB of 8-bit result
//! - Z = cleared if non-zero, unchanged if zero (allows chaining across multiple bytes)
//! - V = undefined in Motorola PRM; on 68000 silicon matches internal decimal overflow
//! - C = decimal carry/borrow

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf BCD Arithmetic Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn abcd_b(state: &mut CpuState, src: u8, dst: u8) -> u8 {
    let x_in = if state.get_x() { 1_u16 } else { 0_u16 };
    let lo1 = (src & 0x0F) as u16;
    let hi1 = (src & 0xF0) as u16;
    let lo2 = (dst & 0x0F) as u16;
    let hi2 = (dst & 0xF0) as u16;
    let newv_lo = lo1 + lo2 + x_in;
    let newv_hi = hi1 + hi2;
    let tmp_newv = newv_hi + newv_lo;
    let mut newv = tmp_newv;
    if newv_lo > 9 {
        newv += 6;
    }
    let cflg = (newv & 0x3F0) > 0x90;
    if cflg {
        newv += 0x60;
    }
    let res = (newv & 0xFF) as u8;
    let vflg = ((tmp_newv & 0x80) == 0) && ((newv & 0x80) != 0);
    let nflg = (res & 0x80) != 0;
    let zflg = if res != 0 { false } else { state.get_z() };
    state.set_ccr_xnzvc(cflg, nflg, zflg, vflg, cflg);
    res
}

#[inline(always)]
pub fn sbcd_b(state: &mut CpuState, src: u8, dst: u8) -> u8 {
    let x_in = if state.get_x() { 1_i16 } else { 0_i16 };
    let s = src as i16;
    let d = dst as i16;
    let newv_lo = (d & 0x0F) - (s & 0x0F) - x_in;
    let newv_hi = (d & 0xF0) - (s & 0xF0);
    let tmp_newv = newv_hi + newv_lo;
    let mut newv = tmp_newv;
    let mut bcd = 0;
    if (newv_lo & 0xF0) != 0 {
        newv -= 6;
        bcd = 6;
    }
    if (((d & 0xFF) - (s & 0xFF) - x_in) & 0x100) != 0 {
        newv -= 0x60;
    }
    let cflg = (((d & 0xFF) - (s & 0xFF) - bcd - x_in) & 0x300) > 0xFF;
    let res = (newv & 0xFF) as u8;
    let vflg = ((tmp_newv & 0x80) != 0) && ((newv & 0x80) == 0);
    let nflg = (res & 0x80) != 0;
    let zflg = if res != 0 { false } else { state.get_z() };
    state.set_ccr_xnzvc(cflg, nflg, zflg, vflg, cflg);
    res
}

#[inline(always)]
pub fn nbcd_b(state: &mut CpuState, src: u8) -> u8 {
    sbcd_b(state, src, 0)
}

// ============================================================================
// Micro-Step ALU Callbacks: ABCD
// ============================================================================

pub fn alu_abcd_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = abcd_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_abcd_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = abcd_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

// ============================================================================
// Micro-Step ALU Callbacks: SBCD
// ============================================================================

pub fn alu_sbcd_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = sbcd_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_sbcd_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = sbcd_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

// ============================================================================
// Micro-Step ALU Callbacks: NBCD
// ============================================================================

pub fn alu_nbcd_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = nbcd_b(state, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_nbcd_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFF) as u8;
    let res = nbcd_b(state, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: ABCD
// ============================================================================

pub static STEPS_ABCD_DN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_abcd_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ABCD_PD_PD: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::READ_SRC_BYTE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_abcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: SBCD
// ============================================================================

pub static STEPS_SBCD_DN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_SBCD_PD_PD: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::READ_SRC_BYTE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: NBCD
// ============================================================================

pub static STEPS_NBCD_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_NBCD_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_D16: [MicroStep; 8] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_IDX: [MicroStep; 9] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Lookup Tables & Decode Functions
// ============================================================================

static NBCD_LOOKUP: [&[MicroStep]; 8] = [
    &STEPS_NBCD_DN,
    &STEPS_NBCD_AI,
    &STEPS_NBCD_PI,
    &STEPS_NBCD_PD,
    &STEPS_NBCD_D16,
    &STEPS_NBCD_IDX,
    &STEPS_NBCD_ABSW,
    &STEPS_NBCD_ABSL,
];

pub const fn decode_nbcd_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(NBCD_LOOKUP[0]),
        2 => Some(NBCD_LOOKUP[1]),
        3 => Some(NBCD_LOOKUP[2]),
        4 => Some(NBCD_LOOKUP[3]),
        5 => Some(NBCD_LOOKUP[4]),
        6 => Some(NBCD_LOOKUP[5]),
        7 => match reg {
            0 => Some(NBCD_LOOKUP[6]),
            1 => Some(NBCD_LOOKUP[7]),
            _ => None,
        },
        _ => None,
    }
}
