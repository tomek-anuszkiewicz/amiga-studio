//! M68000 AND Instruction (`AND <ea>, Dn` and `AND Dn, <ea>`)
//!
//! Performs bitwise AND between a source operand and a destination data register or memory location.
//! CCR flags: N = MSB(res), Z = (res == 0), V = 0, C = 0, X unaffected.

use crate::micro::types::MicroStep;
use crate::state::CpuState;

pub mod dn_ea;
pub mod ea_dn;

// ============================================================================
// Core Leaf ALU AND Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn and_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x80) != 0, res == 0);
    res
}

#[inline(always)]
pub fn and_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x8000) != 0, res == 0);
    res
}

#[inline(always)]
pub fn and_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x8000_0000) != 0, res == 0);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_and_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = and_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_and_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    let res = and_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_and_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = and_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

pub fn alu_and_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

pub fn alu_and_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.destination;
    let res = and_l(state, s, d);
    state.micro.destination = res;
}

// ============================================================================
// Opcode Decoder Function
// ============================================================================

pub const fn decode_and_steps(
    dir: u8,
    size: u8,
    mode: u8,
    reg: u8,
) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        ea_dn::decode_and_ea_dn_steps(size, mode, reg)
    } else {
        dn_ea::decode_and_dn_ea_steps(size, mode, reg)
    }
}
