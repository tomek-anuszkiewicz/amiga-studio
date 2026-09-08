//! JMP (Jump) instruction handler
//!
//! Unconditional program jump to an effective address using control addressing modes:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::types::{flags, MicroAction, MicroStep};
use memory_bus::MemoryBus;

/// JMP (An): 8 CPU clocks / 4 CCKs
pub static STEPS_JMP_AI: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_src_ai),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (d16, An): 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_D16_AN: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_src_d16_an),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (d8, An, Xn): 14 CPU clocks / 7 CCKs
pub static STEPS_JMP_IDX_AN: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_src_idx_an_pure),
        base_clocks: 6,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (xxx).W: 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_ABSW: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_absw),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (xxx).L: 12 CPU clocks / 6 CCKs
pub static STEPS_JMP_ABSL: [MicroStep; 5] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::FETCH_EXTENSION,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_absl_lo),
        base_clocks: 0,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (d16, PC): 10 CPU clocks / 5 CCKs
pub static STEPS_JMP_D16_PC: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_d16_pc),
        base_clocks: 2,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// JMP (d8, PC, Xn): 14 CPU clocks / 7 CCKs
pub static STEPS_JMP_IDX_PC: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(crate::micro::ea::ea_calc_idx_pc_pure),
        base_clocks: 6,
        flags: flags::NONE,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// Compile-time opcode decoder for JMP ($4ED0..=$4EFF)
pub const fn decode_jmp_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_JMP_AI),
        5 => Some(&STEPS_JMP_D16_AN),
        6 => Some(&STEPS_JMP_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_JMP_ABSW),
            1 => Some(&STEPS_JMP_ABSL),
            2 => Some(&STEPS_JMP_D16_PC),
            3 => Some(&STEPS_JMP_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}

/// Resolves control addressing mode target (retained for backward compatibility)
#[inline(always)]
pub fn resolve_control_target(cpu: &Cpu, mode: u8, reg: usize, base_pc: u32) -> u32 {
    match mode {
        2 => cpu.state.read_a(reg),
        5 => {
            let disp = cpu.state.prefetch[0] as i16 as i32;
            cpu.state.read_a(reg).wrapping_add(disp as u32)
        }
        6 => {
            let ext = cpu.state.prefetch[0];
            let base = cpu.state.read_a(reg);
            let disp = (ext & 0xFF) as i8 as i32;
            let xn = crate::micro::ea::read_index_reg(&cpu.state, ext);
            base.wrapping_add(disp as u32).wrapping_add(xn)
        }
        7 => match reg {
            0 => cpu.state.prefetch[0] as i16 as i32 as u32,
            2 => {
                let disp = cpu.state.prefetch[0] as i16 as i32;
                base_pc.wrapping_add(disp as u32)
            }
            3 => {
                let ext = cpu.state.prefetch[0];
                let disp = (ext & 0xFF) as i8 as i32;
                let xn = crate::micro::ea::read_index_reg(&cpu.state, ext);
                base_pc.wrapping_add(disp as u32).wrapping_add(xn)
            }
            _ => 0,
        },
        _ => 0,
    }
}

/// Execution handler for `JMP <ea>`
pub fn op_jmp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let mode = ((cpu.state.ir >> 3) & 7) as u8;
    let reg = (cpu.state.ir & 7) as u8;
    if let Some(steps) = decode_jmp_steps(mode, reg) {
        cpu.state.micro.current_steps = steps;
        cpu.state.micro.reg_src = reg;
        crate::micro::execute_micro_step(cpu, bus)
    } else {
        cpu.state.halted = true;
        StepResult::Halted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_jmp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}
