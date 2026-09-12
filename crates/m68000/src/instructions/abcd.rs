//! M68000 ABCD Instruction (`ABCD Dy, Dx` and `ABCD -(Ay), -(Ax)`)
//!
//! Binary-Coded Decimal addition with extend (X) flag:
//! `Source_BCD + Destination_BCD + X -> Destination_BCD`
//!
//! Flags:
//! - X = C (decimal carry)
//! - N = MSB of 8-bit result
//! - Z = cleared if non-zero, unchanged if zero (allows chaining across multiple bytes)
//! - V = undefined in Motorola PRM; on 68000 silicon matches internal decimal overflow
//! - C = decimal carry

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
// Static Micro-Step Slices: ABCD
// ============================================================================

pub static STEPS_ABCD_DN_DN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_abcd_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ABCD_PD_PD: [MicroStep; 9] = [
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
        alu_fn: Some(alu_abcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_BYTE,
];
