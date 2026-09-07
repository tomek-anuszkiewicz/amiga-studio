//! ROXL (Rotate with Extend Left) instruction handlers and CCR updates
//!
//! Rotates bits to the left through the Extend (X) flag.
//! C flag receives the final state of the X flag. V flag is always cleared.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    data_fc, decode_ea_index, read_ea_operand, size_from_const, SIZE_BYTE, SIZE_LONG, SIZE_WORD,
};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Evaluates ROXL rotate operation and updates condition codes (X, N, Z, V, C)
#[inline(always)]
pub fn execute_roxl(state: &mut CpuState, s: u8, count: u32, val: u32) -> u32 {
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
        state.set_v(false);
        state.set_c(state.get_x());
        // X flag unaffected
        state.set_n((v & msb) != 0);
        state.set_z(v == 0);
        return (val & !mask) | v;
    }

    state.set_v(false);
    for _ in 0..count {
        let carry_in = if state.get_x() { 1 } else { 0 };
        let carry_out = (v & msb) != 0;
        v = ((v << 1) & mask) | carry_in;
        state.set_x(carry_out);
    }
    state.set_c(state.get_x());
    state.set_n((v & msb) != 0);
    state.set_z(v == 0);

    (val & !mask) | v
}

/// Sized wrapper for ROXL
#[inline]
pub fn execute_roxl_sized(state: &mut CpuState, count: u32, val: u32, size: Size) -> u32 {
    let s = match size {
        Size::Byte => SIZE_BYTE,
        Size::Word => SIZE_WORD,
        Size::Long => SIZE_LONG,
    };
    execute_roxl(state, s, count, val)
}

/// Register rotate: `ROXL Dx, Dy` or `ROXL #<data>, Dy`
pub fn op_roxl_reg(
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
            let res = execute_roxl(&mut cpu.state, s, count, val);
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

/// Memory rotate: `ROXL <ea>` (word size, count = 1)
pub fn op_roxl_mem(
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

        let res = execute_roxl(&mut cpu.state, SIZE_WORD, 1, mem_val);
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
