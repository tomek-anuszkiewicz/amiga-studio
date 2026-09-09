//! MOVEM (Move Multiple Registers) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusResult, MemoryBus};

// ============================================================================
// Pure ALU Callbacks: MOVEM
// ============================================================================

/// Latches the 16-bit register mask extension word into movem_mask and resets transfer state
#[inline(always)]
pub fn alu_movem_fetch_mask(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.movem_mask = state.prefetch[0];
    state.micro.movem_state = 0;
}

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const FETCH_MASK_READ: MicroStep = MicroStep {
    step_fn: Cpu::step_fetch_extension_read,
    alu_fn: Some(alu_movem_fetch_mask),
    base_clocks: 2,
};
const FETCH_MASK_FINISH: MicroStep = common::FETCH_EXT_FINISH;

const FETCH_EXT_READ: MicroStep = common::FETCH_EXT_READ;
const FETCH_EXT_FINISH: MicroStep = common::FETCH_EXT_FINISH;

const MOVEM_TRANSFER: MicroStep = MicroStep {
    step_fn: crate::instructions::movem::execute_movem_transfer,
    alu_fn: None,
    base_clocks: 0,
};
const PREFETCH_RETIRE_READ: MicroStep = common::PREFETCH_NEXT_READ;
const PREFETCH_RETIRE_FINISH: MicroStep = common::PREFETCH_NEXT_RETIRE;

// ============================================================================
// Static Step Slices: MOVEM
// ============================================================================

pub static STEPS_MOVEM_AI: [MicroStep; 6] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_PI: [MicroStep; 6] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_PD: [MicroStep; 6] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_D16: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_fetch_extension_read,
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_IDX: [MicroStep; 8] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_ABSW: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_fetch_extension_read,
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_ABSL: [MicroStep; 9] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_fetch_extension_read,
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Cpu::step_fetch_extension_read,
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_PCD16: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_fetch_extension_read,
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
];

pub static STEPS_MOVEM_PCIDX: [MicroStep; 8] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE_READ,
    PREFETCH_RETIRE_FINISH,
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
) -> BusResult<()> {
    let ir = cpu.state.ir;
    let is_reg_to_mem = (ir & 0x0400) == 0;
    let is_long = (ir & 0x0040) != 0;
    let mode = ((ir >> 3) & 7) as u8;
    let reg_ea = (ir & 7) as usize;
    let is_predec = is_reg_to_mem && mode == 4;
    let is_postinc = !is_reg_to_mem && mode == 3;

    let mask = cpu.state.micro.movem_mask;
    let state_raw = cpu.state.micro.movem_state as u32;
    // Bit 0 tracks the 2-phase Color Clock bus cycle for each word transfer:
    // 0 = CCK1 (bus initiation: read attempt or write idle setup)
    // 1 = CCK2 (bus completion: register commit or memory write attempt)
    let is_cck2 = (state_raw & 1) != 0;
    let mut bit_idx = ((state_raw >> 1) & 0x1F) as u8;
    let mut sub_word = ((state_raw >> 6) & 1) as u8;
    let dummy_read_active = ((state_raw >> 7) & 1) != 0;

    // 1. Initial check: mask == 0 and address alignment
    if bit_idx == 0 && sub_word == 0 && !dummy_read_active && !is_cck2 {
        if mask == 0 {
            cpu.state.micro.movem_state = 0;
            cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
            return BusResult::Ready(());
        }
        let ea = cpu.state.micro.ea_addr;
        if (ea & 1) != 0 {
            if is_predec {
                cpu.trigger_address_error_step(ea.wrapping_sub(2), false, false, bus);
            } else if is_postinc {
                cpu.state.write_a(reg_ea, ea.wrapping_add(2));
                cpu.trigger_address_error_step(ea, true, false, bus);
            } else {
                cpu.trigger_address_error_step(ea, !is_reg_to_mem, false, bus);
            }
            return BusResult::Ready(());
        }
    }

    // 2. Dummy read at conclusion of mem-to-reg transfer
    if dummy_read_active {
        let addr = cpu.state.micro.ea_addr & 0x00FF_FFFF;
        if !is_cck2 {
            // CCK1: read word from memory
            match bus.read_word(addr) {
                BusResult::WaitState => BusResult::WaitState,
                BusResult::Ready(data) => {
                    cpu.state.micro.source = data as u32;
                    cpu.state.micro.movem_state = (state_raw | 1) as u16;
                    BusResult::Ready(())
                }
            }
        } else {
            // CCK2: log transaction and conclude
            let fc = crate::micro::types::data_fc(&cpu.state);
            cpu.state.micro.record_bus_transaction(
                true,
                false,
                fc,
                addr,
                BusAccessSize::Word,
                cpu.state.micro.source as u16,
                true,
                true,
            );
            if is_postinc {
                cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
            }
            cpu.state.micro.movem_state = 0;
            cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
            BusResult::Ready(())
        }
    } else {
        // 3. Find next register in mask (if starting new register)
        if sub_word == 0 && !is_cck2 {
            while bit_idx < 16 && (mask & (1 << bit_idx)) == 0 {
                bit_idx += 1;
            }
        }

        // 4. Transfer word if register found
        if bit_idx < 16 {
            if !is_reg_to_mem {
                let addr = if sub_word == 0 {
                    cpu.state.micro.ea_addr
                } else {
                    cpu.state.micro.ea_addr.wrapping_add(2)
                } & 0x00FF_FFFF;

                if !is_cck2 {
                    // CCK1: read word from bus
                    match bus.read_word(addr) {
                        BusResult::WaitState => BusResult::WaitState,
                        BusResult::Ready(data) => {
                            cpu.state.micro.source = data as u32;
                            cpu.state.micro.movem_state =
                                (((bit_idx as u32) << 1) | ((sub_word as u32) << 6) | 1) as u16;
                            BusResult::Ready(())
                        }
                    }
                } else {
                    // CCK2: commit word into register / buffer and advance
                    let fc = crate::micro::types::data_fc(&cpu.state);
                    cpu.state.micro.record_bus_transaction(
                        true,
                        false,
                        fc,
                        addr,
                        BusAccessSize::Word,
                        cpu.state.micro.source as u16,
                        true,
                        true,
                    );
                    if !is_long {
                        let val = (cpu.state.micro.source as i16 as i32) as u32;
                        movem_write_reg(&mut cpu.state, bit_idx, val);
                        cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(2);
                        bit_idx += 1;
                    } else if sub_word == 0 {
                        cpu.state.micro.destination = (cpu.state.micro.source & 0xFFFF) << 16;
                        sub_word = 1;
                    } else {
                        let val = cpu.state.micro.destination | (cpu.state.micro.source & 0xFFFF);
                        movem_write_reg(&mut cpu.state, bit_idx, val);
                        cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(4);
                        sub_word = 0;
                        bit_idx += 1;
                    }
                    cpu.state.micro.movem_state =
                        (((bit_idx as u32) << 1) | ((sub_word as u32) << 6)) as u16;
                    BusResult::Ready(())
                }
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
                let addr_masked = addr & 0x00FF_FFFF;

                if !is_cck2 {
                    // CCK1: idle bus setup
                    cpu.state.micro.movem_state =
                        (((bit_idx as u32) << 1) | ((sub_word as u32) << 6) | 1) as u16;
                    BusResult::Ready(())
                } else {
                    // CCK2: commit write to memory
                    match bus.write_word(addr_masked, data) {
                        BusResult::WaitState => BusResult::WaitState,
                        BusResult::Ready(()) => {
                            let fc = crate::micro::types::data_fc(&cpu.state);
                            cpu.state.micro.record_bus_transaction(
                                false,
                                false,
                                fc,
                                addr_masked,
                                BusAccessSize::Word,
                                data,
                                true,
                                true,
                            );
                            if is_predec {
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
                            cpu.state.micro.movem_state =
                                (((bit_idx as u32) << 1) | ((sub_word as u32) << 6)) as u16;
                            BusResult::Ready(())
                        }
                    }
                }
            }
        } else {
            // 5. Conclude transfers
            if !is_reg_to_mem {
                // Start dummy read on CCK1
                let addr = cpu.state.micro.ea_addr & 0x00FF_FFFF;
                match bus.read_word(addr) {
                    BusResult::WaitState => {
                        cpu.state.micro.movem_state = ((16 << 1) | (1 << 7)) as u16;
                        BusResult::WaitState
                    }
                    BusResult::Ready(data) => {
                        cpu.state.micro.source = data as u32;
                        cpu.state.micro.movem_state = ((16 << 1) | (1 << 7) | 1) as u16;
                        BusResult::Ready(())
                    }
                }
            } else {
                if is_predec {
                    cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
                }
                cpu.state.micro.movem_state = 0;
                cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                BusResult::Ready(())
            }
        }
    }
}


