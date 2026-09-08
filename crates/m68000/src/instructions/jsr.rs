//! JSR (Jump to Subroutine) instruction handler
//!
//! Pushes the long return address onto the stack and jumps to an effective address
//! using control addressing modes: `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`,
//! `(d16, PC)`, `(d8, PC, Xn)`.
//! Execution time: 16 to 22 CPU clocks depending on addressing mode.

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

#[inline(always)]
pub fn alu_jsr_ai(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.read_a(reg_src as usize);
    state.micro.write_buffer = state.pc.wrapping_sub(2);
}

#[inline(always)]
pub fn alu_jsr_d16_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    state.micro.ea_addr = state.read_a(reg_src as usize).wrapping_add(disp as u32);
    state.micro.write_buffer = state.pc;
}

#[inline(always)]
pub fn alu_jsr_idx_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = crate::micro::ea::read_index_reg(state, ext);
    let an = state.read_a(reg_src as usize);
    state.micro.ea_addr = an.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.write_buffer = state.pc;
}

#[inline(always)]
pub fn alu_jsr_absw(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.prefetch[0] as i16 as i32 as u32;
    state.micro.write_buffer = state.pc;
}

#[inline(always)]
pub fn alu_jsr_absl_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.micro.scratch[0] | (state.prefetch[0] as u32);
    state.micro.write_buffer = state.pc;
}

#[inline(always)]
pub fn alu_jsr_d16_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch[0] as i16) as i32;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
    state.micro.write_buffer = state.pc;
}

#[inline(always)]
pub fn alu_jsr_idx_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch[0];
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = crate::micro::ea::read_index_reg(state, ext);
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.write_buffer = state.pc;
}

/// JSR (An): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_AI: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_ai),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (d16, An): 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_D16_AN: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_d16_an),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (d8, An, Xn): 22 CPU clocks / 11 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_IDX_AN: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_idx_an),
        base_clocks: 6,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (xxx).W: 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_ABSW: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_absw),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (xxx).L: 20 CPU clocks / 10 CCKs (3 reads, 2 writes)
pub static STEPS_JSR_ABSL: [MicroStep; 7] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::FETCH_EXTENSION,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_absl_lo),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (d16, PC): 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_D16_PC: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_d16_pc),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// JSR (d8, PC, Xn): 22 CPU clocks / 11 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_IDX_PC: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_jsr_idx_pc),
        base_clocks: 6,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW,
    common::PREFETCH_TARGET_RETIRE,
];

/// Compile-time opcode decoder for JSR ($4E90..=$4EBF)
pub const fn decode_jsr_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_JSR_AI),
        5 => Some(&STEPS_JSR_D16_AN),
        6 => Some(&STEPS_JSR_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_JSR_ABSW),
            1 => Some(&STEPS_JSR_ABSL),
            2 => Some(&STEPS_JSR_D16_PC),
            3 => Some(&STEPS_JSR_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}

/// Execution handler for `JSR <ea>`
pub fn op_jsr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let mode = ((cpu.state.ir >> 3) & 7) as u8;
    let reg = (cpu.state.ir & 7) as u8;
    if let Some(steps) = decode_jsr_steps(mode, reg) {
        cpu.state.micro.current_steps = steps;
        cpu.state.micro.reg_src = reg;
        crate::micro::execute_micro_step(cpu, bus)
    } else {
        cpu.state.halted = true;
        StepResult::Halted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_jsr_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}

pub fn op_jsr_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jsr(cpu, bus)
}
