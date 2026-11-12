//! MOVEM (Move Multiple Registers) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::field;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;
use physical_memory::{AddressBus, BusResult};

const BIT_LOAD_DIRECTION: u16 = 0x0400;
const BIT_LONG_SIZE: u16 = 0x0040;
const BIT_STATE_CCK2: u32 = 1;
const BIT_STATE_REGISTER_SHIFT: u32 = 1;
const MASK_STATE_REGISTER: u32 = 0x1F;
const BIT_STATE_SUB_WORD_SHIFT: u32 = 6;
const MASK_STATE_SUB_WORD: u32 = 1;
const BIT_STATE_DUMMY_SHIFT: u32 = 7;
const MASK_STATE_DUMMY: u32 = 1;
const REGISTER_BANK_COUNT: u8 = crate::state::REGISTER_COUNT as u8;
const REGISTER_MASK_BITS: u8 = 2 * REGISTER_BANK_COUNT;
const FIRST_WORD: u8 = 0;
const SECOND_WORD: u8 = 1;

// ============================================================================
// Pure ALU Callbacks: MOVEM
// ============================================================================

/// Latches the 16-bit register mask extension word into movem_mask and resets transfer state
pub fn alu_movem_fetch_mask(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro_mut().movem_mask = state.prefetch();
    state.micro_mut().movem_state = 0;
}

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const FETCH_MASK_READ: MicroStep = MicroStep {
    bus_fn: Some(Cpu::step_fetch_extension_read),
    alu_fn: Some(alu_movem_fetch_mask),
    base_clocks: MicroStep::CCK_CLOCKS,
};
const FETCH_MASK_FINISH: MicroStep = common::FETCH_EXT_FINISH;
const FETCH_MASK_FINISH_AI: MicroStep = MicroStep {
    bus_fn: Some(Cpu::step_fetch_extension_finish),
    alu_fn: Some(ea::ea_calc_src_ai),
    base_clocks: MicroStep::CCK_CLOCKS,
};

const MOVEM_TRANSFER: MicroStep = MicroStep {
    bus_fn: Some(crate::instructions::movem::execute_movem_transfer),
    alu_fn: None,
    base_clocks: 0,
};

// ============================================================================
// Static Step Slices: MOVEM
// ============================================================================

pub static STEPS_MOVEM_AI: [MicroStep; 5] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH_AI,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_PI: [MicroStep; 5] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH_AI,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_PD: [MicroStep; 5] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH_AI,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_D16_AN: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_IDX_AN: [MicroStep; 8] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_ABSW: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_ABSL: [MicroStep; 9] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_D16_PC: [MicroStep; 7] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVEM_IDX_PC: [MicroStep; 8] = [
    FETCH_MASK_READ,
    FETCH_MASK_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MOVEM_TRANSFER,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Opcode Decoder: MOVEM
// ============================================================================

/// Compile-time opcode decoder for MOVEM
pub const fn decode_movem_steps(
    is_reg_to_mem: bool,
    mode: u8,
    reg: u8,
) -> Option<&'static [MicroStep]> {
    if is_reg_to_mem {
        match mode {
            mode::INDIRECT => Some(&STEPS_MOVEM_AI),
            mode::PREDECREMENT => Some(&STEPS_MOVEM_PD),
            mode::DISPLACEMENT => Some(&STEPS_MOVEM_D16_AN),
            mode::INDEXED => Some(&STEPS_MOVEM_IDX_AN),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_MOVEM_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_MOVEM_ABSL),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            mode::INDIRECT => Some(&STEPS_MOVEM_AI),
            mode::POSTINCREMENT => Some(&STEPS_MOVEM_PI),
            mode::DISPLACEMENT => Some(&STEPS_MOVEM_D16_AN),
            mode::INDEXED => Some(&STEPS_MOVEM_IDX_AN),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_MOVEM_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_MOVEM_ABSL),
                extended_mode::PC_DISPLACEMENT => Some(&STEPS_MOVEM_D16_PC),
                extended_mode::PC_INDEXED => Some(&STEPS_MOVEM_IDX_PC),
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
        if bit_idx < REGISTER_BANK_COUNT {
            state.a_long((REGISTER_BANK_COUNT - 1 - bit_idx) as usize)
        } else {
            state.d_long((REGISTER_MASK_BITS - 1 - bit_idx) as usize)
        }
    } else if bit_idx < REGISTER_BANK_COUNT {
        state.d_long(bit_idx as usize)
    } else {
        state.a_long((bit_idx - REGISTER_BANK_COUNT) as usize)
    }
}

#[inline(always)]
fn movem_write_reg(state: &mut CpuState, bit_idx: u8, val: u32) {
    if bit_idx < REGISTER_BANK_COUNT {
        state.set_d_long(bit_idx as usize, val);
    } else {
        state.set_a_long((bit_idx - REGISTER_BANK_COUNT) as usize, val);
    }
}

/// Executes a single bus step of the MOVEM multi-register transfer state machine
pub fn execute_movem_transfer(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()> {
    let ir = cpu.state.ir();
    let is_reg_to_mem = (ir & BIT_LOAD_DIRECTION) == 0;
    let is_long = (ir & BIT_LONG_SIZE) != 0;
    let mode = ((ir >> field::BIT_EA_MODE_SHIFT) & field::MASK_EA_MODE as u16) as u8;
    let reg_ea = (ir & field::MASK_REGISTER as u16) as usize;
    let is_predec = is_reg_to_mem && mode == mode::PREDECREMENT;
    let is_postinc = !is_reg_to_mem && mode == mode::POSTINCREMENT;

    let mask = cpu.state.micro().movem_mask;
    let state_raw = cpu.state.micro().movem_state as u32;
    // Bit 0 tracks the 2-phase Color Clock bus cycle for each word transfer:
    // 0 = CCK1 (bus initiation: read attempt or write idle setup)
    // 1 = CCK2 (bus completion: register commit or memory write attempt)
    let is_cck2 = (state_raw & BIT_STATE_CCK2) != 0;
    let mut bit_idx = ((state_raw >> BIT_STATE_REGISTER_SHIFT) & MASK_STATE_REGISTER) as u8;
    let mut sub_word = ((state_raw >> BIT_STATE_SUB_WORD_SHIFT) & MASK_STATE_SUB_WORD) as u8;
    let dummy_read_active = ((state_raw >> BIT_STATE_DUMMY_SHIFT) & MASK_STATE_DUMMY) != 0;

    // 1. Initial check: mask == 0 and address alignment
    let is_transfer_start =
        bit_idx == 0 && sub_word == FIRST_WORD && !dummy_read_active && !is_cck2;
    if is_transfer_start {
        if mask == 0 && is_reg_to_mem {
            // An empty store mask proceeds directly to the prefetch read in this CCK.
            return match cpu.step_prefetch_next_read(bus) {
                BusResult::WaitState => BusResult::WaitState,
                BusResult::Ready(()) => {
                    cpu.state.micro_mut().micro_step = cpu.state.micro().micro_step.wrapping_add(2);
                    BusResult::Ready(())
                }
            };
        }
        let ea = cpu.state.micro().ea_addr;
        if mask != 0 && (ea & 1) != 0 {
            if is_predec {
                cpu.trigger_address_error(ea.wrapping_sub(2), false, false);
            } else if is_postinc {
                cpu.state.set_a_long(reg_ea, ea.wrapping_add(2));
                cpu.trigger_address_error(ea, true, false);
            } else {
                cpu.trigger_address_error(ea, !is_reg_to_mem, false);
            }
            return BusResult::Ready(());
        }
    }

    // 2. Dummy read at conclusion of mem-to-reg transfer
    if dummy_read_active {
        let addr = cpu.state.micro().ea_addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        if !is_cck2 {
            // CCK1: read word from memory
            match bus.read_word(addr) {
                BusResult::WaitState => BusResult::WaitState,
                BusResult::Ready(data) => {
                    cpu.state.micro_mut().source = data as u32;
                    cpu.state.micro_mut().movem_state = (state_raw | BIT_STATE_CCK2) as u16;
                    BusResult::Ready(())
                }
            }
        } else {
            if is_postinc {
                cpu.state.set_a_long(reg_ea, cpu.state.micro().ea_addr);
            }
            cpu.state.micro_mut().movem_state = 0;
            cpu.state.micro_mut().micro_step = cpu.state.micro().micro_step.wrapping_add(1);
            BusResult::Ready(())
        }
    } else {
        // 3. Find next register in mask (if starting new register)
        let is_register_start = sub_word == FIRST_WORD && !is_cck2;
        if is_register_start {
            while bit_idx < REGISTER_MASK_BITS && (mask & (1 << bit_idx)) == 0 {
                bit_idx += 1;
            }
        }

        // 4. Transfer word if register found
        if bit_idx < REGISTER_MASK_BITS {
            if !is_reg_to_mem {
                let addr = if sub_word == FIRST_WORD {
                    cpu.state.micro().ea_addr
                } else {
                    cpu.state.micro().ea_addr.wrapping_add(2)
                } & physical_memory::MASK_PHYSICAL_ADDRESS;

                if !is_cck2 {
                    // CCK1: read word from bus
                    match bus.read_word(addr) {
                        BusResult::WaitState => BusResult::WaitState,
                        BusResult::Ready(data) => {
                            cpu.state.micro_mut().source = data as u32;
                            cpu.state.micro_mut().movem_state =
                                (((bit_idx as u32) << BIT_STATE_REGISTER_SHIFT)
                                    | ((sub_word as u32) << BIT_STATE_SUB_WORD_SHIFT)
                                    | BIT_STATE_CCK2) as u16;
                            BusResult::Ready(())
                        }
                    }
                } else {
                    if !is_long {
                        let val = (cpu.state.micro().source as i16 as i32) as u32;
                        movem_write_reg(&mut cpu.state, bit_idx, val);
                        cpu.state.micro_mut().ea_addr = cpu.state.micro().ea_addr.wrapping_add(2);
                        bit_idx += 1;
                    } else if sub_word == FIRST_WORD {
                        cpu.state.micro_mut().destination =
                            (cpu.state.micro().source & 0xFFFF) << 16;
                        sub_word = SECOND_WORD;
                    } else {
                        let val =
                            cpu.state.micro().destination | (cpu.state.micro().source & 0xFFFF);
                        movem_write_reg(&mut cpu.state, bit_idx, val);
                        cpu.state.micro_mut().ea_addr = cpu.state.micro().ea_addr.wrapping_add(4);
                        sub_word = FIRST_WORD;
                        bit_idx += 1;
                    }
                    cpu.state.micro_mut().movem_state = (((bit_idx as u32)
                        << BIT_STATE_REGISTER_SHIFT)
                        | ((sub_word as u32) << BIT_STATE_SUB_WORD_SHIFT))
                        as u16;
                    BusResult::Ready(())
                }
            } else {
                let reg_val = movem_read_reg(&cpu.state, is_predec, bit_idx);
                let (addr, data) = if is_predec {
                    if !is_long || sub_word == FIRST_WORD {
                        (
                            cpu.state.micro().ea_addr.wrapping_sub(2),
                            (reg_val & 0xFFFF) as u16,
                        )
                    } else {
                        (
                            cpu.state.micro().ea_addr.wrapping_sub(4),
                            (reg_val >> 16) as u16,
                        )
                    }
                } else if !is_long {
                    (cpu.state.micro().ea_addr, (reg_val & 0xFFFF) as u16)
                } else if sub_word == FIRST_WORD {
                    (cpu.state.micro().ea_addr, (reg_val >> 16) as u16)
                } else {
                    (
                        cpu.state.micro().ea_addr.wrapping_add(2),
                        (reg_val & 0xFFFF) as u16,
                    )
                };
                let addr_masked = addr & physical_memory::MASK_PHYSICAL_ADDRESS;

                if !is_cck2 {
                    // CCK1: idle bus setup
                    cpu.state.micro_mut().movem_state =
                        (((bit_idx as u32) << BIT_STATE_REGISTER_SHIFT)
                            | ((sub_word as u32) << BIT_STATE_SUB_WORD_SHIFT)
                            | BIT_STATE_CCK2) as u16;
                    BusResult::Ready(())
                } else {
                    // CCK2: commit write to memory
                    match bus.write_word(addr_masked, data) {
                        BusResult::WaitState => BusResult::WaitState,
                        BusResult::Ready(()) => {
                            if is_predec {
                                if !is_long {
                                    cpu.state.micro_mut().ea_addr =
                                        cpu.state.micro().ea_addr.wrapping_sub(2);
                                    bit_idx += 1;
                                } else if sub_word == FIRST_WORD {
                                    sub_word = SECOND_WORD;
                                } else {
                                    cpu.state.micro_mut().ea_addr =
                                        cpu.state.micro().ea_addr.wrapping_sub(4);
                                    sub_word = FIRST_WORD;
                                    bit_idx += 1;
                                }
                            } else if !is_long {
                                cpu.state.micro_mut().ea_addr =
                                    cpu.state.micro().ea_addr.wrapping_add(2);
                                bit_idx += 1;
                            } else if sub_word == FIRST_WORD {
                                sub_word = SECOND_WORD;
                            } else {
                                cpu.state.micro_mut().ea_addr =
                                    cpu.state.micro().ea_addr.wrapping_add(4);
                                sub_word = FIRST_WORD;
                                bit_idx += 1;
                            }
                            cpu.state.micro_mut().movem_state = (((bit_idx as u32)
                                << BIT_STATE_REGISTER_SHIFT)
                                | ((sub_word as u32) << BIT_STATE_SUB_WORD_SHIFT))
                                as u16;
                            // The final write CCK completes the loop; no extra idle precedes prefetch.
                            if sub_word == FIRST_WORD && (mask as u32) >> bit_idx == 0 {
                                if is_predec {
                                    cpu.state.set_a_long(reg_ea, cpu.state.micro().ea_addr);
                                }
                                cpu.state.micro_mut().movem_state = 0;
                                cpu.state.micro_mut().micro_step =
                                    cpu.state.micro().micro_step.wrapping_add(1);
                            }
                            BusResult::Ready(())
                        }
                    }
                }
            }
        } else {
            // 5. Conclude transfers
            if !is_reg_to_mem {
                // Start dummy read on CCK1
                let addr = cpu.state.micro().ea_addr & physical_memory::MASK_PHYSICAL_ADDRESS;
                match bus.read_word(addr) {
                    BusResult::WaitState => {
                        cpu.state.micro_mut().movem_state = ((u32::from(REGISTER_MASK_BITS)
                            << BIT_STATE_REGISTER_SHIFT)
                            | (MASK_STATE_DUMMY << BIT_STATE_DUMMY_SHIFT))
                            as u16;
                        BusResult::WaitState
                    }
                    BusResult::Ready(data) => {
                        cpu.state.micro_mut().source = data as u32;
                        cpu.state.micro_mut().movem_state =
                            ((u32::from(REGISTER_MASK_BITS) << BIT_STATE_REGISTER_SHIFT)
                                | (MASK_STATE_DUMMY << BIT_STATE_DUMMY_SHIFT)
                                | BIT_STATE_CCK2) as u16;
                        BusResult::Ready(())
                    }
                }
            } else {
                if is_predec {
                    cpu.state.set_a_long(reg_ea, cpu.state.micro().ea_addr);
                }
                cpu.state.micro_mut().movem_state = 0;
                cpu.state.micro_mut().micro_step = cpu.state.micro().micro_step.wrapping_add(1);
                BusResult::Ready(())
            }
        }
    }
}
