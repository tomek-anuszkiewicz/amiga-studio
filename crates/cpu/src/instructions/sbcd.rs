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

use super::{BCD_DIGIT_CORRECTION, BCD_HIGH_CORRECTION, MASK_BCD_HIGH_DIGIT, MASK_BCD_LOW_DIGIT};
const BIT_BCD_BORROW: i16 = 0x100;
const MASK_BCD_CARRY: i16 = 0x300;
use super::BIT_BYTE_SIGN;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Core Leaf BCD Arithmetic Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn sbcd_b(state: &mut CpuState, src: u8, dst: u8) -> u8 {
    let x_in = if state.is_x() { 1_i16 } else { 0_i16 };
    let s = src as i16;
    let d = dst as i16;
    let newv_lo = (d & MASK_BCD_LOW_DIGIT as i16) - (s & MASK_BCD_LOW_DIGIT as i16) - x_in;
    let newv_hi = (d & MASK_BCD_HIGH_DIGIT as i16) - (s & MASK_BCD_HIGH_DIGIT as i16);
    let tmp_newv = newv_hi + newv_lo;
    let mut newv = tmp_newv;
    let mut bcd = 0;
    if (newv_lo & MASK_BCD_HIGH_DIGIT as i16) != 0 {
        newv -= BCD_DIGIT_CORRECTION as i16;
        bcd = BCD_DIGIT_CORRECTION as i16;
    }
    if (((d & 0xFF) - (s & 0xFF) - x_in) & BIT_BCD_BORROW) != 0 {
        newv -= BCD_HIGH_CORRECTION as i16;
    }
    let cflg = (((d & 0xFF) - (s & 0xFF) - bcd - x_in) & MASK_BCD_CARRY) > 0xFF;
    let res = (newv & 0xFF) as u8;
    let vflg =
        ((tmp_newv & i16::from(BIT_BYTE_SIGN)) != 0) && ((newv & i16::from(BIT_BYTE_SIGN)) == 0);
    let nflg = (res & BIT_BYTE_SIGN) != 0;
    let zflg = if res != 0 { false } else { state.is_z() };
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
    let s = (state.micro().source & 0xFF) as u8;
    let d = (state.micro().destination & 0xFF) as u8;
    let res = sbcd_b(state, s, d);
    state.micro_mut().destination = (state.micro().destination & !0xFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: SBCD
// ============================================================================

pub static STEPS_SBCD_DN_DN: [MicroStep; 3] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_dn_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub static STEPS_SBCD_PD_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dual_pd_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_ADDR1_BYTE,
    common::BUS_READ_IDLE,
    common::READ_ADDR2_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sbcd_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_BYTE,
];
