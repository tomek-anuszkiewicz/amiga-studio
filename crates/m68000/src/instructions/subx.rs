//! M68000 SUBX Instruction (`SUBX Dy, Dx` and `SUBX -(Ay), -(Ax)`)
//!
//! Quirk note: In SUBX, the Z flag is cleared if the result is non-zero,
//! but remains unchanged if the result is zero (preserving chained multi-precision zero status).

use crate::core::{Cpu, StepResult};
use crate::micro::types::{flags, MicroAction, MicroStep, Size};
use crate::state::CpuState;
use memory_bus::MemoryBus;

/// Evaluates pure SUBX arithmetic and updates CCR flags (X, N, Z, V, C)
#[inline(always)]
pub fn execute_subx(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let x = if state.get_x() { 1 } else { 0 };
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x as u8);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            let z = if res != 0 { false } else { state.get_z() };
            state.set_ccr_xnzvc(c, n, z, v, c);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x as u16);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            let z = if res != 0 { false } else { state.get_z() };
            state.set_ccr_xnzvc(c, n, z, v, c);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let s = src;
            let d = dst;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
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

pub fn alu_subx_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_subx(state, s, d, Size::Byte);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_subx_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_subx(state, s, d, Size::Word);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_subx_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = execute_subx(state, s, d, Size::Long);
    state.set_d_long(reg_dst as usize, res);
}

// ============================================================================
// Micro-Step Callbacks: Predecrement Memory -(Ay), -(Ax)
// ============================================================================

pub fn alu_subx_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = (state.micro.last_read & 0xFF) as u32;
    let res = execute_subx(state, s, d, Size::Byte);
    state.micro.write_buffer = res & 0xFF;
}

pub fn alu_subx_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.micro.last_read as u32;
    let res = execute_subx(state, s, d, Size::Word);
    state.micro.write_buffer = res & 0xFFFF;
}

pub fn latch_dst_and_calc_subx_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ax_val = state.micro.scratch[2] | ((state.micro.last_read as u32) << 16);
    let ay_val = state.micro.scratch[1];
    let res = execute_subx(state, ay_val, ax_val, Size::Long);
    state.micro.scratch[1] = res;
    state.micro.ea_addr = state.micro.scratch[0].wrapping_add(2);
    state.micro.write_buffer = res & 0xFFFF;
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_SUBX_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_subx_b_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_SUBX_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_subx_w_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_SUBX_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_subx_l_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_SUBX_B_PD_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(crate::instructions::addx::ea_calc_src_pd_b_2clocks), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(crate::instructions::addx::latch_src_b_and_calc_dst_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_subx_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_SUBX_W_PD_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(crate::instructions::addx::ea_calc_src_pd_w_2clocks), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(crate::instructions::addx::latch_src_w_and_calc_dst_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_subx_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_SUBX_L_PD_PD: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(crate::instructions::addx::ea_calc_src_pd_l_2clocks), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(crate::instructions::addx::latch_src_lo_and_read_src_hi), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(crate::instructions::addx::latch_src_hi_and_calc_dst_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(crate::instructions::addx::latch_dst_lo_and_read_dst_hi), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusWriteWord, alu_fn: Some(latch_dst_and_calc_subx_l), base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: Some(crate::instructions::addx::set_write_hi), base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

/// Decodes the micro-step sequence for SUBX based on addressing type and size
pub const fn decode_subx_steps(is_memory: bool, size: u8) -> Option<&'static [MicroStep]> {
    if !is_memory {
        match size {
            0 => Some(&STEPS_SUBX_B_DN_DN),
            1 => Some(&STEPS_SUBX_W_DN_DN),
            2 => Some(&STEPS_SUBX_L_DN_DN),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_SUBX_B_PD_PD),
            1 => Some(&STEPS_SUBX_W_PD_PD),
            2 => Some(&STEPS_SUBX_L_PD_PD),
            _ => None,
        }
    }
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_subx_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_subx_b_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_subx_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_subx_l_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_subx_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_subx_w_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}
