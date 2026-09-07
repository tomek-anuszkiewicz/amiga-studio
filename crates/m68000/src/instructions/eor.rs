//! M68000 EOR Instruction (`EOR Dn, <ea>`)
//!
//! Provides cycle-exact Color Clock execution handlers and branchless
//! size-specialized ALU logic functions (`eor_b`, `eor_w`, `eor_l`, `execute_eor`).

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::*;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// Leaf ALU EOR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn eor_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d ^ s;
    state.set_n((res & 0x80) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn eor_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d ^ s;
    state.set_n((res & 0x8000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn eor_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d ^ s;
    state.set_n((res & 0x8000_0000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

pub fn execute_eor(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = eor_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = eor_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => eor_l(state, src, dst),
    }
}

// ============================================================================
// EOR: Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_eor_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_eor_dn_to_ea(cpu, bus, s, m)
}

#[inline(always)]
fn exec_eor_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    if m == EA_DN {
        if s == SIZE_LONG {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_s = ((cpu.state.ir >> 9) & 7) as usize;
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let s_val = cpu.state.d[reg_s];
                    let d_val = cpu.state.d[reg_d];
                    let res = execute_eor(&mut cpu.state, s_val, d_val, Size::Long);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            let reg_s = ((cpu.state.ir >> 9) & 7) as usize;
            let reg_d = (cpu.state.ir & 7) as usize;
            let s_val = cpu.state.d[reg_s];
            let d_val = cpu.state.d[reg_d];
            let res = execute_eor(&mut cpu.state, s_val, d_val, size);
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
            let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
            let d_val = cpu.state.d[reg_d];
            let res = execute_eor(&mut cpu.state, d_val, mem_val, size);
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
                StepResult::StepCompleted
            } else {
                let addr = cpu.state.micro.scratch[0];
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(addr, hi, BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                StepResult::StepCompleted
            }
        } else {
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            let addr = cpu.state.micro.scratch[0];
            let val = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, bus_size, fc));
            cpu.state.micro.mark_scratch_prefetch_retire();
            StepResult::StepCompleted
        }
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_eor_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_l_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

pub fn op_eor_w_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_eor_dn_to_ea(cpu, bus)
}

