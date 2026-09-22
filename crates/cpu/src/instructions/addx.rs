//! M68000 ADDX Instruction (`ADDX Dy, Dx` and `ADDX -(Ay), -(Ax)`)
//!
//! Quirk note: In ADDX, the Z flag is cleared if the result is non-zero,
//! but remains unchanged if the result is zero (preserving chained multi-precision zero status).

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Core Leaf ALU Addition with Extend Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn addx_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let x = if state.is_x() { 1 } else { 0 };
    let (res1, c1) = d.overflowing_add(s);
    let (res, c2) = res1.overflowing_add(x);
    let c = c1 || c2;
    let v = ((!(s ^ d) & (d ^ res)) & 0x80) != 0;
    let n = (res & 0x80) != 0;
    let z = if res != 0 { false } else { state.is_z() };
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn addx_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let x = if state.is_x() { 1 } else { 0 };
    let (res1, c1) = d.overflowing_add(s);
    let (res, c2) = res1.overflowing_add(x);
    let c = c1 || c2;
    let v = ((!(s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = if res != 0 { false } else { state.is_z() };
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn addx_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let x = if state.is_x() { 1 } else { 0 };
    let (res1, c1) = d.overflowing_add(s);
    let (res, c2) = res1.overflowing_add(x);
    let c = c1 || c2;
    let v = ((!(s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
    let n = (res & 0x8000_0000) != 0;
    let z = if res != 0 { false } else { state.is_z() };
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

// ============================================================================
// Micro-Step Callbacks: Register-to-Register (Dy, Dx)
// ============================================================================

pub fn alu_addx_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = addx_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_addx_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = addx_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_addx_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = addx_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

// ============================================================================
// Micro-Step Callbacks: Predecrement Memory -(Ay), -(Ax)
// ============================================================================

pub fn alu_addx_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = addx_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

pub fn alu_addx_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = addx_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

pub fn alu_addx_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    let res = addx_l(state, s, d);
    state.micro.destination = res;
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_ADDX_B_DN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_b_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ADDX_W_DN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_w_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ADDX_L_DN_DN: [MicroStep; 3] = [
    common::ALU_IDLE_4CLK,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_l_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_ADDX_B_PD_PD: [MicroStep; 9] = [
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
        alu_fn: Some(alu_addx_b_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_BYTE,
];

pub static STEPS_ADDX_W_PD_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_ADDR1_WORD,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_ADDR2_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_WORD,
];

pub static STEPS_ADDX_L_PD_PD: [MicroStep; 15] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l_split),
        base_clocks: 2,
    },
    common::READ_SRC_WORD,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::latch_src_lo_and_read_src_hi),
        base_clocks: 2,
    },
    common::READ_SRC_SPLIT_HIGH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_l_split),
        base_clocks: 2,
    },
    common::READ_DST_WORD,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::latch_dst_lo_and_read_dst_hi),
        base_clocks: 2,
    },
    common::READ_DST_SPLIT_HIGH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_addx_l_mem),
        base_clocks: 2,
    },
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_PD_LONG_LOW,
    common::PREFETCH_IRC_READ,
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_PD_LONG_HIGH,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for ADDX
// ============================================================================

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
