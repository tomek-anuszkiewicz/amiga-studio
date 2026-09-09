//! M68000 ADDX Instruction (`ADDX Dy, Dx` and `ADDX -(Ay), -(Ax)`)
//!
//! Quirk note: In ADDX, the Z flag is cleared if the result is non-zero,
//! but remains unchanged if the result is zero (preserving chained multi-precision zero status).

use crate::core::Cpu;
use crate::micro::ea;
use crate::micro::types::{MicroStep, Size};
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

pub fn alu_addx_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    let res = execute_addx(state, s, d, Size::Byte);
    state.micro.destination = res;
}

pub fn alu_addx_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    let res = execute_addx(state, s, d, Size::Word);
    state.micro.destination = res;
}

pub fn latch_dst_and_calc_addx_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let res = execute_addx(
        state,
        state.micro.source,
        state.micro.destination,
        Size::Long,
    );
    state.micro.destination = res;
    state.micro.ea_addr = state.micro.ea_high.wrapping_add(2);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_ADDX_B_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_b_dn_dn),
        base_clocks: 2,
    },
    crate::micro::common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_ADDX_W_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_w_dn_dn),
        base_clocks: 2,
    },
    crate::micro::common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_ADDX_L_DN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_l_dn_dn),
        base_clocks: 2,
    },
    crate::micro::common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_ADDX_B_PD_PD: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    crate::micro::common::READ_SRC_BYTE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    crate::micro::common::READ_DST_BYTE,
    crate::micro::common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_addx_b_mem),
        base_clocks: 2,
    },
    crate::micro::common::PREFETCH_IRC_FINISH,
    crate::micro::common::BUS_WRITE_IDLE,
    crate::micro::common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_ADDX_W_PD_PD: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    crate::micro::common::READ_SRC_WORD,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    crate::micro::common::READ_DST_WORD,
    crate::micro::common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_addx_w_mem),
        base_clocks: 2,
    },
    crate::micro::common::PREFETCH_IRC_FINISH,
    crate::micro::common::BUS_WRITE_IDLE,
    crate::micro::common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_ADDX_L_PD_PD: [MicroStep; 15] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l_split),
        base_clocks: 2,
    },
    crate::micro::common::READ_SRC_WORD,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::latch_src_lo_and_read_src_hi),
        base_clocks: 2,
    },
    crate::micro::common::READ_SRC_SPLIT_HIGH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::calc_dst_pd_l),
        base_clocks: 2,
    },
    crate::micro::common::READ_DST_WORD,
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::latch_dst_lo_and_read_dst_hi),
        base_clocks: 2,
    },
    crate::micro::common::READ_DST_SPLIT_HIGH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(latch_dst_and_calc_addx_l),
        base_clocks: 2,
    },
    crate::micro::common::BUS_WRITE_IDLE,
    crate::micro::common::WRITE_DST_WORD,
    crate::micro::common::PREFETCH_IRC_READ,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_finish),
        alu_fn: Some(ea::set_write_hi),
        base_clocks: 2,
    },
    crate::micro::common::BUS_WRITE_IDLE,
    crate::micro::common::WRITE_DST_WORD_RETIRE,
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
