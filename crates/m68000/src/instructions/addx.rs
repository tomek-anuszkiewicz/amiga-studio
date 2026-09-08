//! M68000 ADDX Instruction (`ADDX Dy, Dx` and `ADDX -(Ay), -(Ax)`)
//!
//! Quirk note: In ADDX, the Z flag is cleared if the result is non-zero,
//! but remains unchanged if the result is zero (preserving chained multi-precision zero status).

use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep, Size};
use crate::state::CpuState;

/// Evaluates pure ADDX arithmetic and updates CCR flags (X, N, Z, V, C)
#[inline(always)]
pub fn execute_addx(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let x = if state.get_x() { 1 } else { 0 };
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x as u8);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            let z = if res != 0 { false } else { state.get_z() };
            state.set_ccr_xnzvc(c, n, z, v, c);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x as u16);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            let z = if res != 0 { false } else { state.get_z() };
            state.set_ccr_xnzvc(c, n, z, v, c);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let s = src;
            let d = dst;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
            let n = (res & 0x8000_0000) != 0;
            let z = if res != 0 { false } else { state.get_z() };
            state.set_ccr_xnzvc(c, n, z, v, c);
            res
        }
    }
}

// ============================================================================
// Micro-Step Callbacks: Register-to-Register (Dy, Dx)
// ============================================================================

pub fn alu_addx_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_addx(state, s, d, Size::Byte);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_addx_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_addx(state, s, d, Size::Word);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_addx_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_addx(state, s, d, Size::Long);
    state.set_d_long(reg_dst as usize, res);
}

// ============================================================================
// Micro-Step Callbacks: Predecrement Memory -(Ay), -(Ax)
// ============================================================================

pub fn ea_calc_src_pd_b_2clocks(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    ea::ea_calc_src_pd_b(state, reg_src, 0);
    state.micro.internal_clocks = 2;
}

pub fn latch_src_b_and_calc_dst_pd_b(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = (state.micro.last_read & 0xFF) as u32;
    ea::ea_calc_dst_pd_b(state, 0, reg_dst);
}

pub fn alu_addx_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = (state.micro.last_read & 0xFF) as u32;
    let res = execute_addx(state, s, d, Size::Byte);
    state.micro.write_buffer = res & 0xFF;
}

pub fn ea_calc_src_pd_w_2clocks(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    ea::ea_calc_src_pd_w(state, reg_src, 0);
    state.micro.internal_clocks = 2;
}

pub fn latch_src_w_and_calc_dst_pd_w(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = state.micro.last_read as u32;
    ea::ea_calc_dst_pd_w(state, 0, reg_dst);
}

pub fn alu_addx_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.micro.last_read as u32;
    let res = execute_addx(state, s, d, Size::Word);
    state.micro.write_buffer = res & 0xFFFF;
}

pub fn ea_calc_src_pd_l_2clocks(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let an = state.read_a(reg_src as usize);
    let low_addr = an.wrapping_sub(2);
    state.write_a(reg_src as usize, low_addr);
    state.micro.scratch[0] = an.wrapping_sub(4);
    state.micro.ea_addr = low_addr;
    state.micro.internal_clocks = 2;
}

pub fn latch_src_lo_and_read_src_hi(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.write_a(reg_src as usize, state.micro.scratch[0]);
    state.micro.scratch[1] = state.micro.last_read as u32;
    state.micro.ea_addr = state.micro.scratch[0];
}

pub fn latch_src_hi_and_calc_dst_pd_l(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] |= (state.micro.last_read as u32) << 16;
    let ax = state.read_a(reg_dst as usize);
    let low_addr = ax.wrapping_sub(2);
    state.write_a(reg_dst as usize, low_addr);
    state.micro.scratch[0] = ax.wrapping_sub(4);
    state.micro.ea_addr = low_addr;
}

pub fn latch_dst_lo_and_read_dst_hi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.write_a(reg_dst as usize, state.micro.scratch[0]);
    state.micro.scratch[2] = state.micro.last_read as u32;
    state.micro.ea_addr = state.micro.scratch[0];
}

pub fn latch_dst_and_calc_addx_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ax_val = state.micro.scratch[2] | ((state.micro.last_read as u32) << 16);
    let ay_val = state.micro.scratch[1];
    let res = execute_addx(state, ay_val, ax_val, Size::Long);
    state.micro.scratch[1] = res;
    state.micro.ea_addr = state.micro.scratch[0].wrapping_add(2);
    state.micro.write_buffer = res & 0xFFFF;
}

pub fn set_write_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.micro.scratch[0];
    state.micro.write_buffer = (state.micro.scratch[1] >> 16) & 0xFFFF;
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_ADDX_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addx_b_dn_dn), base_clocks: 4 },
];

pub static STEPS_ADDX_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addx_w_dn_dn), base_clocks: 4 },
];

pub static STEPS_ADDX_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addx_l_dn_dn), base_clocks: 4 },
];

pub static STEPS_ADDX_B_PD_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea_calc_src_pd_b_2clocks), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(latch_src_b_and_calc_dst_pd_b), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addx_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ADDX_W_PD_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea_calc_src_pd_w_2clocks), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(latch_src_w_and_calc_dst_pd_w), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addx_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ADDX_L_PD_PD: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea_calc_src_pd_l_2clocks), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(latch_src_lo_and_read_src_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(latch_src_hi_and_calc_dst_pd_l), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(latch_dst_lo_and_read_dst_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWord, alu_fn: Some(latch_dst_and_calc_addx_l), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: Some(set_write_hi), base_clocks: 4 },
];

/// Decodes the micro-step sequence for ADDX based on addressing type and size
pub const fn decode_addx_steps(is_memory: bool, size: u8) -> Option<&'static [MicroStep]> {
    if !is_memory {
        match size {
            0 => Some(&STEPS_ADDX_B_DN_DN),
            1 => Some(&STEPS_ADDX_W_DN_DN),
            2 => Some(&STEPS_ADDX_L_DN_DN),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ADDX_B_PD_PD),
            1 => Some(&STEPS_ADDX_W_PD_PD),
            2 => Some(&STEPS_ADDX_L_PD_PD),
            _ => None,
        }
    }
}


