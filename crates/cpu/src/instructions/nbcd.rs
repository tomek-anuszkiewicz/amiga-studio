//! M68000 NBCD Instruction (`NBCD <ea>`)
//!
//! Binary-Coded Decimal negation with extend (X) flag:
//! `0 - Destination_BCD - X -> Destination_BCD`
//!
//! Flags:
//! - X = C (decimal borrow)
//! - N = MSB of 8-bit result
//! - Z = cleared if non-zero, unchanged if zero (allows chaining across multiple bytes)
//! - V = undefined in Motorola PRM; on 68000 silicon matches internal decimal overflow
//! - C = decimal borrow

use crate::instructions::sbcd::sbcd_b;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Core Leaf BCD Arithmetic Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn nbcd_b(state: &mut CpuState, src: u8) -> u8 {
    sbcd_b(state, src, 0)
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
// Static Micro-Step Slices: NBCD
// ============================================================================

pub static STEPS_NBCD_DN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_NBCD_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_D16: [MicroStep; 8] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_IDX: [MicroStep; 9] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_nbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_NBCD_ABSL: [MicroStep; 10] = [
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
