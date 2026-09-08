//! PEA (Push Effective Address) instruction handler
//!
//! Pushes effective address onto the stack using control addressing modes:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.
//! Execution time: 12 to 20 CPU clocks depending on addressing mode.

use crate::micro::common;
use crate::core::Cpu;
use crate::micro::types::MicroStep;

/// PEA (An): 12 CPU clocks / 6 CCKs (1 read, 2 writes)
pub static STEPS_PEA_AI: [MicroStep; 4] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_ai),
        base_clocks: 0,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d16, An): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_D16_AN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Cpu::step_fetch_extension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_d16_an),
        base_clocks: 4,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d8, An, Xn): 20 CPU clocks / 10 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_IDX_AN: [MicroStep; 5] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_idx_an),
        base_clocks: 4,
    },
    common::FETCH_EXTENSION,
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (xxx).W: 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_ABSW: [MicroStep; 4] = [
    MicroStep {
        step_fn: Cpu::step_fetch_extension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_absw),
        base_clocks: 4,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (xxx).L: 20 CPU clocks / 10 CCKs (3 reads, 2 writes)
pub static STEPS_PEA_ABSL: [MicroStep; 5] = [
    MicroStep {
        step_fn: Cpu::step_fetch_extension,
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Cpu::step_fetch_extension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_absl_lo),
        base_clocks: 4,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d16, PC): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_D16_PC: [MicroStep; 4] = [
    MicroStep {
        step_fn: Cpu::step_fetch_extension,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_d16_pc),
        base_clocks: 4,
    },
    common::RMW_PREFETCH_SCRATCH,
    common::PUSH_STACK_HIGH,
    common::PUSH_STACK_LOW_RETIRE,
];

/// PEA (d8, PC, Xn): 20 CPU clocks / 10 CCKs (2 reads, 2 writes)
pub static STEPS_PEA_IDX_PC: [MicroStep; 5] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(crate::micro::ea::ea_calc_pea_idx_pc),
        base_clocks: 4,
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


