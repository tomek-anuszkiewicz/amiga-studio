//! MOVEM (Move Multiple Registers) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::engine::{initiate_read_cycle, initiate_write_cycle, trigger_address_error_step};
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Pure ALU Callbacks: MOVEM
// ============================================================================

/// Latches the 16-bit register mask extension word into scratch[2] and resets transfer state
#[inline(always)]
pub fn alu_movem_fetch_mask(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[2] = state.prefetch[0] as u32;
    state.micro.scratch[3] = 0;
}

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const FETCH_MASK: MicroStep = MicroStep {
    action: MicroAction::FetchExtension,
    alu_fn: Some(alu_movem_fetch_mask),
    base_clocks: 4,
    flags: flags::READ | flags::PROGRAM_SPACE,
};
const FETCH_EXT: MicroStep = MicroStep {
    action: MicroAction::FetchExtension,
    alu_fn: None,
    base_clocks: 4,
    flags: flags::READ | flags::PROGRAM_SPACE,
};
const MOVEM_TRANSFER: MicroStep = MicroStep {
    action: MicroAction::MovemTransfer,
    alu_fn: None,
    base_clocks: 0,
    flags: flags::NONE,
};
const PREFETCH_RETIRE: MicroStep = common::RETIRE_STANDARD;

// ============================================================================
// Static Step Slices: MOVEM
// ============================================================================

pub static STEPS_MOVEM_AI: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PI: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PD: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_D16: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_IDX: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
        flags: flags::NONE,
    },
    FETCH_EXT,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_ABSW: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_ABSL: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PCD16: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PCIDX: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
        flags: flags::NONE,
    },
    FETCH_EXT,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

// ============================================================================
// Opcode Decoder: MOVEM
// ============================================================================

/// Compile-time opcode decoder for MOVEM
pub const fn decode_movem_steps(is_reg_to_mem: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if is_reg_to_mem {
        match mode {
            2 => Some(&STEPS_MOVEM_AI),
            4 => Some(&STEPS_MOVEM_PD),
            5 => Some(&STEPS_MOVEM_D16),
            6 => Some(&STEPS_MOVEM_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEM_ABSW),
                1 => Some(&STEPS_MOVEM_ABSL),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            2 => Some(&STEPS_MOVEM_AI),
            3 => Some(&STEPS_MOVEM_PI),
            5 => Some(&STEPS_MOVEM_D16),
            6 => Some(&STEPS_MOVEM_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEM_ABSW),
                1 => Some(&STEPS_MOVEM_ABSL),
                2 => Some(&STEPS_MOVEM_PCD16),
                3 => Some(&STEPS_MOVEM_PCIDX),
                _ => None,
            },
            _ => None,
        }
    }
}

// ============================================================================
// Multi-Register Transfer Execution: MOVEM
// ============================================================================

#[inline(always)]
fn movem_read_reg(state: &CpuState, is_predec: bool, bit_idx: u8) -> u32 {
    if is_predec {
        if bit_idx < 8 {
            state.read_a((7 - bit_idx) as usize)
        } else {
            state.d_long((15 - bit_idx) as usize)
        }
    } else if bit_idx < 8 {
        state.d_long(bit_idx as usize)
    } else {
        state.read_a((bit_idx - 8) as usize)
    }
}

#[inline(always)]
fn movem_write_reg(state: &mut CpuState, bit_idx: u8, val: u32) {
    if bit_idx < 8 {
        state.set_d_long(bit_idx as usize, val);
    } else {
        state.write_a((bit_idx - 8) as usize, val);
    }
}

/// Executes a single bus step of the MOVEM multi-register transfer state machine
pub fn execute_movem_transfer(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> Option<StepResult> {
    let ir = cpu.state.ir;
    let is_reg_to_mem = (ir & 0x0400) == 0;
    let is_long = (ir & 0x0040) != 0;
    let mode = ((ir >> 3) & 7) as u8;
    let reg_ea = (ir & 7) as usize;
    let is_predec = is_reg_to_mem && mode == 4;
    let is_postinc = !is_reg_to_mem && mode == 3;
    let fc = crate::micro::types::data_fc(&cpu.state);

    let mask = cpu.state.micro.scratch[2] as u16;
    let state_raw = cpu.state.micro.scratch[3];
    let bus_in_flight = (state_raw & 1) != 0;
    let mut bit_idx = ((state_raw >> 1) & 0x1F) as u8;
    let mut sub_word = ((state_raw >> 6) & 1) as u8;
    let dummy_read_active = ((state_raw >> 7) & 1) != 0;

    // 1. Initial check: mask == 0 and address alignment
    if !bus_in_flight && bit_idx == 0 && sub_word == 0 && !dummy_read_active {
        if mask == 0 {
            cpu.state.micro.scratch[3] = 0;
            cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
            return None;
        }
        let ea = cpu.state.micro.ea_addr;
        if (ea & 1) != 0 {
            if is_predec {
                return Some(trigger_address_error_step(cpu, ea.wrapping_sub(2), false, false, bus));
            } else if is_postinc {
                cpu.state.write_a(reg_ea, ea.wrapping_add(2));
                return Some(trigger_address_error_step(cpu, ea, true, false, bus));
            } else {
                return Some(trigger_address_error_step(cpu, ea, !is_reg_to_mem, false, bus));
            }
        }
    }

    // 2. Process finished bus cycle
    if bus_in_flight {
        if dummy_read_active {
            if is_postinc {
                cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
            }
            cpu.state.micro.scratch[3] = 0;
            cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
            return None;
        }

        if !is_reg_to_mem {
            if !is_long {
                let val = (cpu.state.micro.last_read as i16 as i32) as u32;
                movem_write_reg(&mut cpu.state, bit_idx, val);
                cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(2);
                bit_idx += 1;
            } else if sub_word == 0 {
                cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                sub_word = 1;
            } else {
                let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                movem_write_reg(&mut cpu.state, bit_idx, val);
                cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(4);
                sub_word = 0;
                bit_idx += 1;
            }
        } else if is_predec {
            if !is_long {
                cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_sub(2);
                bit_idx += 1;
            } else if sub_word == 0 {
                sub_word = 1;
            } else {
                cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_sub(4);
                sub_word = 0;
                bit_idx += 1;
            }
        } else if !is_long {
            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(2);
            bit_idx += 1;
        } else if sub_word == 0 {
            sub_word = 1;
        } else {
            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(4);
            sub_word = 0;
            bit_idx += 1;
        }
    }

    // 3. Find next register in mask
    if sub_word == 0 {
        while bit_idx < 16 && (mask & (1 << bit_idx)) == 0 {
            bit_idx += 1;
        }
    }

    // 4. Initiate transfer if register found
    if bit_idx < 16 {
        let new_state = 1 | ((bit_idx as u32) << 1) | ((sub_word as u32) << 6);
        cpu.state.micro.scratch[3] = new_state;

        if !is_reg_to_mem {
            let addr = if sub_word == 0 {
                cpu.state.micro.ea_addr
            } else {
                cpu.state.micro.ea_addr.wrapping_add(2)
            };
            return Some(initiate_read_cycle(cpu, bus, addr, memory_bus::BusAccessSize::Word, fc));
        } else {
            let reg_val = movem_read_reg(&cpu.state, is_predec, bit_idx);
            let (addr, data) = if is_predec {
                if !is_long || sub_word == 0 {
                    (cpu.state.micro.ea_addr.wrapping_sub(2), (reg_val & 0xFFFF) as u16)
                } else {
                    (cpu.state.micro.ea_addr.wrapping_sub(4), (reg_val >> 16) as u16)
                }
            } else if !is_long {
                (cpu.state.micro.ea_addr, (reg_val & 0xFFFF) as u16)
            } else if sub_word == 0 {
                (cpu.state.micro.ea_addr, (reg_val >> 16) as u16)
            } else {
                (cpu.state.micro.ea_addr.wrapping_add(2), (reg_val & 0xFFFF) as u16)
            };
            return Some(initiate_write_cycle(cpu, bus, addr, data, memory_bus::BusAccessSize::Word, fc));
        }
    }

    // 5. Conclude transfers
    if !is_reg_to_mem {
        let new_state = 1 | (16 << 1) | (1 << 7);
        cpu.state.micro.scratch[3] = new_state;
        Some(initiate_read_cycle(cpu, bus, cpu.state.micro.ea_addr, memory_bus::BusAccessSize::Word, fc))
    } else {
        if is_predec {
            cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
        }
        cpu.state.micro.scratch[3] = 0;
        cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
        None
    }
}

