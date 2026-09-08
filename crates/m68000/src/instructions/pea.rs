//! PEA (Push Effective Address) instruction handler
//!
//! Pushes effective address onto the stack using control addressing modes:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.
//! Execution time: 12 to 20 CPU clocks depending on addressing mode.

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::types::{flags, MicroAction, MicroStep};
use memory_bus::MemoryBus;

/// PEA (An): 12 CPU clocks / 6 CCKs (1 read, 2 writes)
pub static STEPS_PEA_AI: [MicroStep; 4] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_ai),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d16, An): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_D16_AN: [MicroStep; 4] = [
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_d16_an),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d8, An, Xn): 20 CPU clocks / 10 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_IDX_AN: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_idx_an),
        base_clocks: 4,
        flags: flags::NONE,
    },
    common::FETCH_EXTENSION,
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (xxx).W: 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_ABSW: [MicroStep; 4] = [
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_absw),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (xxx).L: 20 CPU clocks / 10 CCKs (3 reads, 2 writes)
pub static STEPS_PEA_ABSL: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_absl_lo),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d16, PC): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_D16_PC: [MicroStep; 4] = [
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_d16_pc),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d8, PC, Xn): 20 CPU clocks / 10 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_IDX_PC: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_idx_pc),
        base_clocks: 4,
        flags: flags::NONE,
    },
    common::FETCH_EXTENSION,
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// Compile-time opcode decoder for PEA ($4840..=$487F)
pub const fn decode_pea_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_PEA_AI),
        5 => Some(&STEPS_PEA_D16_AN),
        6 => Some(&STEPS_PEA_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_PEA_ABSW),
            1 => Some(&STEPS_PEA_ABSL),
            2 => Some(&STEPS_PEA_D16_PC),
            3 => Some(&STEPS_PEA_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}

/// Execution handler for `PEA <ea>`
pub fn op_pea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let mode = ((cpu.state.ir >> 3) & 7) as u8;
    let reg = (cpu.state.ir & 7) as u8;
    if let Some(steps) = decode_pea_steps(mode, reg) {
        cpu.state.micro.current_steps = steps;
        cpu.state.micro.reg_src = reg;
        crate::micro::execute_micro_step(cpu, bus)
    } else {
        cpu.state.halted = true;
        StepResult::Halted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_pea_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_inv5(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_inv6(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_inv7(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}

pub fn op_pea_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_pea(cpu, bus)
}
