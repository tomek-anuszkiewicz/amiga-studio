//! LSR (Logical Shift Right) instruction handlers and CCR updates
//!
//! Shifts bits to the right, shifting 0 into MSB and the LSB into C and X flags.
//! V flag is always cleared.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    data_fc, decode_ea_index, read_ea_operand, size_from_const, SIZE_BYTE, SIZE_LONG, SIZE_WORD,
};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Evaluates LSR shift operation and updates condition codes (X, N, Z, V, C)
#[inline(always)]
pub fn execute_lsr(state: &mut CpuState, s: u8, count: u32, val: u32) -> u32 {
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

    if count == 0 {
        // X flag unaffected, V=0, C=0
        state.set_ccr_nz_clear_vc((v & msb) != 0, v == 0);
        return (val & !mask) | v;
    }

    let last_out = if count < width {
        let bit = (v & (1 << (count - 1))) != 0;
        v >>= count;
        bit
    } else if count == width {
        let bit = (v & msb) != 0;
        v = 0;
        bit
    } else {
        v = 0;
        false
    };
    state.set_ccr_xnzvc(last_out, (v & msb) != 0, v == 0, false, last_out);

    (val & !mask) | v
}

/// Sized wrapper for LSR
#[inline]
pub fn execute_lsr_sized(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let s = match size {
        Size::Byte => SIZE_BYTE,
        Size::Word => SIZE_WORD,
        Size::Long => SIZE_LONG,
    };
    execute_lsr(state, s, count, val)
}

/// Register shift: `LSR Dx, Dy` or `LSR #<data>, Dy`
pub fn op_lsr_reg(
    cpu: &mut Cpu,
    _bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let is_reg_count = ((ir >> 5) & 1) != 0;

    match cpu.state.micro.micro_step {
        0 => {
            let count = if is_reg_count {
                let reg_cnt = ((ir >> 9) & 7) as usize;
                cpu.state.d_long(reg_cnt) & 63
            } else {
                let raw = ((ir >> 9) & 7) as u32;
                if raw == 0 {
                    8
                } else {
                    raw
                }
            };

            let reg_dst = (ir & 7) as usize;
            let val = cpu.state.d_long(reg_dst);
            let res = execute_lsr(&mut cpu.state, s, count, val);
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

/// Memory shift: `LSR <ea>` (word size, count = 1)
pub fn op_lsr_mem(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let fc = data_fc(cpu);
    let phase = cpu.state.micro.scratch[2];

    if phase == 0 {
        let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, SIZE_WORD, m) {
            Ok(v) => v,
            Err(res) => return res,
        };

        let res = execute_lsr(&mut cpu.state, SIZE_WORD, 1, mem_val);
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

// --- Specialized Opcode Forwarders ---

pub fn op_lsr_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_w_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_w_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_reg(cpu, bus)
}

pub fn op_lsr_w_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

pub fn op_lsr_w_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_lsr_mem(cpu, bus)
}

