//! CMPM (Compare Memory) Instruction Handlers
//!
//! Compares memory operands via postincrement: `CMPM (Ay)+, (Ax)+`.
//! Evaluates ((Ax) - (Ay)) and updates N, Z, V, and C flags.
//! Neither memory location is modified. Extend (X) flag is unaffected.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

/// Latches source byte into scratch[1] and sets destination address register indirect with postincrement
pub fn ea_calc_dst_pi_b_latch_src(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = (state.micro.last_read & 0xFF) as u32;
    let ax = state.read_a(reg_dst as usize);
    state.micro.ea_addr = ax;
    let inc = if reg_dst == 7 { 2 } else { 1 };
    state.write_a(reg_dst as usize, ax.wrapping_add(inc));
}

/// Latches source word into scratch[1] and sets destination address register indirect with postincrement
pub fn ea_calc_dst_pi_w_latch_src(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = state.micro.last_read as u32;
    let ax = state.read_a(reg_dst as usize);
    state.micro.ea_addr = ax;
    state.write_a(reg_dst as usize, ax.wrapping_add(2));
}

/// Latches source longword into scratch[2] and sets destination address register indirect with postincrement
pub fn ea_calc_dst_pi_l_latch_src(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[2] = state.micro.scratch[1];
    let ax = state.read_a(reg_dst as usize);
    state.micro.ea_addr = ax;
    state.write_a(reg_dst as usize, ax.wrapping_add(4));
}

/// ALU compare callback for Byte
pub fn alu_cmpm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    crate::instructions::cmp::cmp_b(state, s, d);
}

/// ALU compare callback for Word
pub fn alu_cmpm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFFFF) as u16;
    let d = state.micro.last_read;
    crate::instructions::cmp::cmp_w(state, s, d);
}

/// ALU compare callback for Long
pub fn alu_cmpm_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[2];
    let d = state.micro.scratch[1];
    crate::instructions::cmp::cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_CMPM_B: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea_calc_dst_pi_b_latch_src), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpm_b), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPM_W: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea_calc_dst_pi_w_latch_src), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpm_w), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPM_L: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea_calc_dst_pi_l_latch_src), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpm_l), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

/// Decodes the micro-step sequence for CMPM based on size (0 = Byte, 1 = Word, 2 = Long)
pub const fn decode_cmpm_steps(size: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => Some(&STEPS_CMPM_B),
        1 => Some(&STEPS_CMPM_W),
        2 => Some(&STEPS_CMPM_L),
        _ => None,
    }
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_cmpm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_cmpm_b_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}

pub fn op_cmpm_l_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}

pub fn op_cmpm_w_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}
