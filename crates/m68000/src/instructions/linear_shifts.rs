//! Cycle-exact linear M68000 shift and rotate instructions
//! (ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR)
//!
//! Provides flat, branchless concrete instruction handlers eliminating
//! cascaded runtime branching in the hot instruction dispatch loop (Rule 2.6).

use super::linear_ea::*;
use crate::core::{Cpu, StepResult};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// Core Shift & Rotate Computation Engine
// ============================================================================

#[inline(always)]
fn execute_shift_rotate(
    state: &mut CpuState,
    s: u8,
    is_left: bool,
    shift_type: u8,
    count: u32,
    val: u32,
) -> u32 {
    let width = match s {
        SIZE_BYTE => 8u32,
        SIZE_WORD => 16u32,
        _ => 32u32,
    };
    let mask = match s {
        SIZE_BYTE => 0xFFu32,
        SIZE_WORD => 0xFFFFu32,
        _ => 0xFFFF_FFFFu32,
    };
    let msb = 1u32 << (width - 1);
    let mut v = val & mask;

    // Count == 0: Special condition code semantics
    if count == 0 {
        state.set_v(false);
        if shift_type == 2 {
            // ROXd: Carry takes the current Extend bit
            state.set_c(state.get_x());
        } else {
            // ASd, LSd, ROd: Carry cleared
            state.set_c(false);
        }
        // X flag is unaffected
        state.set_n((v & msb) != 0);
        state.set_z(v == 0);
        return (val & !mask) | v;
    }

    match shift_type {
        0 => {
            // ASd (Arithmetic Shift)
            if is_left {
                // ASL
                let mut last_out = false;
                let mut overflow = false;
                for _ in 0..count {
                    let old_msb = (v & msb) != 0;
                    last_out = old_msb;
                    v = (v << 1) & mask;
                    let new_msb = (v & msb) != 0;
                    if old_msb != new_msb {
                        overflow = true;
                    }
                }
                state.set_x(last_out);
                state.set_c(last_out);
                state.set_v(overflow);
            } else {
                // ASR
                state.set_v(false);
                let mut last_out = false;
                if count <= width {
                    for _ in 0..count {
                        last_out = (v & 1) != 0;
                        let sign_bit = v & msb;
                        v = (v >> 1) | sign_bit;
                    }
                    state.set_x(last_out);
                    state.set_c(last_out);
                } else {
                    // Count exceeds width: silicon exhaustion
                    if (v & msb) != 0 {
                        v = mask;
                    } else {
                        v = 0;
                    }
                    state.set_x(false);
                    state.set_c(false);
                }
            }
        }
        1 => {
            // LSd (Logical Shift)
            state.set_v(false);
            if is_left {
                // LSL
                if count < width {
                    let last_out = (v & (1 << (width - count))) != 0;
                    v = (v << count) & mask;
                    state.set_x(last_out);
                    state.set_c(last_out);
                } else if count == width {
                    let last_out = (v & 1) != 0;
                    v = 0;
                    state.set_x(last_out);
                    state.set_c(last_out);
                } else {
                    v = 0;
                    state.set_x(false);
                    state.set_c(false);
                }
            } else {
                // LSR
                if count < width {
                    let last_out = (v & (1 << (count - 1))) != 0;
                    v >>= count;
                    state.set_x(last_out);
                    state.set_c(last_out);
                } else if count == width {
                    let last_out = (v & msb) != 0;
                    v = 0;
                    state.set_x(last_out);
                    state.set_c(last_out);
                } else {
                    v = 0;
                    state.set_x(false);
                    state.set_c(false);
                }
            }
        }
        2 => {
            // ROXd (Rotate through Extend)
            state.set_v(false);
            if is_left {
                // ROXL
                for _ in 0..count {
                    let carry_in = if state.get_x() { 1 } else { 0 };
                    let carry_out = (v & msb) != 0;
                    v = ((v << 1) & mask) | carry_in;
                    state.set_x(carry_out);
                }
            } else {
                // ROXR
                for _ in 0..count {
                    let carry_in = if state.get_x() { msb } else { 0 };
                    let carry_out = (v & 1) != 0;
                    v = (v >> 1) | carry_in;
                    state.set_x(carry_out);
                }
            }
            state.set_c(state.get_x());
        }
        _ => {
            // ROd (Rotate without Extend) - X flag is untouched
            state.set_v(false);
            let k = count % width;
            if is_left {
                // ROL
                let last_out = if k == 0 {
                    (v & 1) != 0
                } else {
                    v = ((v << k) | (v >> (width - k))) & mask;
                    (v & 1) != 0
                };
                state.set_c(last_out);
            } else {
                // ROR
                let last_out = if k == 0 {
                    (v & msb) != 0
                } else {
                    v = ((v >> k) | (v << (width - k))) & mask;
                    (v & msb) != 0
                };
                state.set_c(last_out);
            }
        }
    }

    state.set_n((v & msb) != 0);
    state.set_z(v == 0);

    (val & !mask) | v
}

// ============================================================================
// Register Shifts & Rotates
// ============================================================================

pub fn op_shift_reg(
    cpu: &mut Cpu,
    _bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let is_left = ((ir >> 8) & 1) != 0;
    let is_reg_count = ((ir >> 5) & 1) != 0;
    let shift_type = ((ir >> 3) & 3) as u8;

    match cpu.state.micro.micro_step {
        0 => {
            let count = if is_reg_count {
                let reg_cnt = ((ir >> 9) & 7) as usize;
                cpu.state.d[reg_cnt] & 63
            } else {
                let raw = ((ir >> 9) & 7) as u32;
                if raw == 0 {
                    8
                } else {
                    raw
                }
            };

            let reg_dst = (ir & 7) as usize;
            let val = cpu.state.d[reg_dst];
            let res = execute_shift_rotate(&mut cpu.state, s, is_left, shift_type, count, val);
            let size = size_from_const(s);
            cpu.write_d_reg(reg_dst, res, size);

            cpu.state.micro.scratch[0] = count;
            cpu.initiate_prefetch();
            StepResult::StepCompleted
        }
        1 => {
            let count = cpu.state.micro.scratch[0];
            let idle_clocks = (if s == SIZE_LONG {
                4 + (2 * count)
            } else {
                2 + (2 * count)
            }) as u16;
            cpu.record_internal_clocks(idle_clocks);
            StepResult::StepCompleted
        }
        2 => {
            cpu.state.ir = cpu.state.prefetch[0];
            cpu.state.prefetch[0] = cpu.state.micro.last_read;
            cpu.state.pc = cpu.state.pc.wrapping_add(2);
            cpu.state.micro.reset();
            StepResult::InstructionCompleted
        }
        _ => unreachable!(),
    }
}

// ============================================================================
// Memory Shifts & Rotates (Class 0 Read-Modify-Write, Word Size, Count = 1)
// ============================================================================

pub fn op_shift_mem(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let shift_type = ((ir >> 9) & 3) as u8;
    let is_left = ((ir >> 8) & 1) != 0;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    let fc = data_fc(cpu);
    let phase = cpu.state.micro.scratch[2];

    if phase == 0 {
        let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, SIZE_WORD, m) {
            Ok(v) => v,
            Err(res) => return res,
        };

        let res =
            execute_shift_rotate(&mut cpu.state, SIZE_WORD, is_left, shift_type, 1, mem_val);
        cpu.state.micro.scratch[1] = res;
        cpu.initiate_prefetch();
        cpu.state.micro.scratch[2] = 1;
        StepResult::StepCompleted
    } else {
        cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
        let addr = cpu.state.micro.scratch[0];
        let val = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
        cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, BusAccessSize::Word, fc));
        cpu.state.micro.mark_scratch_prefetch_retire();
        StepResult::StepCompleted
    }
}
