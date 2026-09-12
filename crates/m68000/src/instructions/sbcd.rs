//! M68000 SBCD Instruction (`SBCD Dy, Dx` and `SBCD -(Ay), -(Ax)`)
//!
//! Binary-Coded Decimal subtraction with extend (X) flag:
//! `Destination_BCD - Source_BCD - X -> Destination_BCD`
//!
//! Flags:
//! - X = C (decimal borrow)
//! - N = MSB of 8-bit result
//! - Z = cleared if non-zero, unchanged if zero (allows chaining across multiple bytes)
//! - V = undefined in Motorola PRM; on 68000 silicon matches internal decimal overflow
//! - C = decimal borrow

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf BCD Arithmetic Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

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
// Static Micro-Step Slices: SBCD
// ============================================================================

pub static STEPS_SBCD_DN_DN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_SBCD_PD_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dual_pd_b),
        base_clocks: 2,
    },
    common::READ_ADDR1_BYTE,
    common::BUS_READ_IDLE,
    common::READ_ADDR2_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_BYTE,
];
