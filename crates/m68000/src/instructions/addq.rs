//! M68000 ADDQ Instruction (`ADDQ #<data>, <ea>`)
//!
//! Adds an immediate 3-bit value (1..8) to a register or memory location.
//! When target is an address register An, CCR flags are unaffected.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::add::execute_add;
use crate::instructions::ea::*;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

pub fn op_addq(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_addq(cpu, bus, s, m)
}

#[inline(always)]
fn exec_addq(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let raw_data = ((cpu.state.ir >> 9) & 7) as u32;
    let imm = if raw_data == 0 { 8 } else { raw_data };

    if m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let reg_a = (cpu.state.ir & 7) as usize;
                let a = cpu.state.read_a(reg_a);
                let res = a.wrapping_add(imm);
                cpu.state.write_a(reg_a, res);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else if m == EA_DN {
        let size = size_from_const(s);
        if s == SIZE_LONG {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let d = cpu.state.d[reg_d];
                    let res = execute_add(&mut cpu.state, imm, d, Size::Long, true);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            let reg_d = (cpu.state.ir & 7) as usize;
            let d = cpu.state.d[reg_d];
            let res = execute_add(&mut cpu.state, imm, d, size, true);
            cpu.write_d_reg(reg_d, res, size);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        // Memory destination: Class 0 RMW
        let size = size_from_const(s);
        let bus_size = bus_size_from_const(s);
        let fc = data_fc(cpu);

        let phase = cpu.state.micro.scratch[2];
        if phase == 0 {
            let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };
            let res = execute_add(&mut cpu.state, imm, mem_val, size, true);
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

pub fn op_addq_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_l_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

pub fn op_addq_w_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_addq(cpu, bus)
}

