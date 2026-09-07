//! M68000 NOT Instruction (`NOT <ea>`)
//!
//! Performs bitwise NOT (one's complement) on a destination data register or memory location.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::*;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// Leaf ALU NOT Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn not_b(state: &mut CpuState, d: u8) -> u8 {
    let res = !d;
    state.set_n((res & 0x80) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn not_w(state: &mut CpuState, d: u16) -> u16 {
    let res = !d;
    state.set_n((res & 0x8000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn not_l(state: &mut CpuState, d: u32) -> u32 {
    let res = !d;
    state.set_n((res & 0x8000_0000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

pub fn execute_not(state: &mut CpuState, val: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = not_b(state, (val & 0xFF) as u8);
            (val & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = not_w(state, (val & 0xFFFF) as u16);
            (val & !0xFFFF) | (res as u32)
        }
        Size::Long => not_l(state, val),
    }
}

// ============================================================================
// NOT: <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_not(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let s = ((cpu.state.ir >> 6) & 3) as u8;
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    if m == EA_DN {
        if s == SIZE_LONG {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(2);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let val = cpu.state.d[reg_d];
                    let res = !val;
                    cpu.state.set_n((res & 0x8000_0000) != 0);
                    cpu.state.set_z(res == 0);
                    cpu.state.set_v(false);
                    cpu.state.set_c(false);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            let reg_d = (cpu.state.ir & 7) as usize;
            let val = cpu.state.d[reg_d];
            let (res, n, z) = if s == SIZE_BYTE {
                let r = !(val as u8);
                (r as u32, (r & 0x80) != 0, r == 0)
            } else {
                let r = !(val as u16);
                (r as u32, (r & 0x8000) != 0, r == 0)
            };
            cpu.state.set_n(n);
            cpu.state.set_z(z);
            cpu.state.set_v(false);
            cpu.state.set_c(false);
            cpu.write_d_reg(reg_d, res, size);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        let phase = cpu.state.micro.scratch[2];
        if phase == 0 {
            let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };
            let (res, n, z) = match s {
                SIZE_BYTE => {
                    let r = !(mem_val as u8);
                    (r as u32, (r & 0x80) != 0, r == 0)
                }
                SIZE_WORD => {
                    let r = !(mem_val as u16);
                    (r as u32, (r & 0x8000) != 0, r == 0)
                }
                _ => {
                    let r = !mem_val;
                    (r, (r & 0x8000_0000) != 0, r == 0)
                }
            };
            cpu.state.set_n(n);
            cpu.state.set_z(z);
            cpu.state.set_v(false);
            cpu.state.set_c(false);

            cpu.state.micro.scratch[1] = res;
            cpu.initiate_prefetch();
            cpu.state.micro.scratch[2] = 1;
            return StepResult::StepCompleted;
        }

        if s == SIZE_LONG {
            if phase == 1 {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(addr2, lo, BusAccessSize::Word, fc));
                cpu.state.micro.scratch[2] = 2;
                return StepResult::StepCompleted;
            } else {
                let addr = cpu.state.micro.scratch[0];
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(addr, hi, BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return StepResult::StepCompleted;
            }
        } else {
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            let addr = cpu.state.micro.scratch[0];
            let val = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, bus_size, fc));
            cpu.state.micro.mark_scratch_prefetch_retire();
            return StepResult::StepCompleted;
        }
    }
}
