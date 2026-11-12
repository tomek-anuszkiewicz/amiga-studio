//! JMP (Jump) instruction handler
//!
//! Unconditional program jump to an effective address using control addressing modes:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.

use crate::micro::common;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::Cpu;

use crate::micro::ea;

/// JMP (An): 8 CPU clocks / 4 CCKs
pub static STEPS_JMP_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_control_target_opcode_read),
        alu_fn: Some(crate::micro::ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (d16, An): 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_D16_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(crate::micro::ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (d8, An, Xn): 14 CPU clocks / 7 CCKs
pub static STEPS_JMP_IDX_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(crate::micro::ea::ea_calc_src_idx_an),
        base_clocks: ea::CONTROL_INDEX_CALC_CLOCKS,
    },
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (xxx).W: 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_ABSW: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(crate::micro::ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (xxx).L: 12 CPU clocks / 6 CCKs
pub static STEPS_JMP_ABSL: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_control_target_opcode_read),
        alu_fn: Some(crate::micro::ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (d16, PC): 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_D16_PC: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(crate::micro::ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JMP (d8, PC, Xn): 14 CPU clocks / 7 CCKs
pub static STEPS_JMP_IDX_PC: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(crate::micro::ea::ea_calc_idx_pc),
        base_clocks: ea::CONTROL_INDEX_CALC_CLOCKS,
    },
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// Compile-time opcode decoder for JMP ($4ED0..=$4EFF)
pub const fn decode_jmp_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::INDIRECT => Some(&STEPS_JMP_AI),
        mode::DISPLACEMENT => Some(&STEPS_JMP_D16_AN),
        mode::INDEXED => Some(&STEPS_JMP_IDX_AN),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_JMP_ABSW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_JMP_ABSL),
            extended_mode::PC_DISPLACEMENT => Some(&STEPS_JMP_D16_PC),
            extended_mode::PC_INDEXED => Some(&STEPS_JMP_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}
