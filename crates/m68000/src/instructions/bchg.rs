//! BCHG (Bit Change) instruction handler and operation logic
//!
//! Tests and inverts a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit was 0 prior to change, cleared if bit was 1). Other condition codes unaffected.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    data_fc, decode_ea_index, prefetch_extension, read_ea_operand, EA_DN, SIZE_BYTE,
};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Evaluates CCR updates and flips the bit for BCHG
#[inline]
pub fn execute_bchg(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) -> u32 {
    let bit_idx = if is_register {
        bit_num % 32
    } else {
        bit_num % 8
    };
    let bit_mask = 1 << bit_idx;
    let bit_val = (val & bit_mask) != 0;
    state.set_z(!bit_val);
    val ^ bit_mask
}

/// Dynamic bit change: `BCHG Dn, <ea>`
pub fn op_bchg_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d[reg_bit];

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d[reg_d];
                let res = execute_bchg(&mut cpu.state, bit_num, val, true);
                cpu.write_d_reg(reg_d, res, Size::Long);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            1 => {
                cpu.record_internal_clocks(4);
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
    } else {
        let phase = cpu.state.micro.scratch[2];
        if phase == 0 {
            let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, SIZE_BYTE, m) {
                Ok(v) => v,
                Err(res) => return res,
            };
            let res = execute_bchg(&mut cpu.state, bit_num, val, false);
            cpu.state.micro.scratch[1] = res;
            cpu.initiate_prefetch();
            cpu.state.micro.scratch[2] = 1;
            StepResult::StepCompleted
        } else {
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            let addr = cpu.state.micro.scratch[0];
            let val = (cpu.state.micro.scratch[1] & 0xFF) as u16;
            let fc = data_fc(cpu);
            cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, BusAccessSize::Byte, fc));
            cpu.state.micro.mark_scratch_prefetch_retire();
            StepResult::StepCompleted
        }
    }
}

/// Static bit change: `BCHG #<data>, <ea>`
pub fn op_bchg_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let bit_num = (cpu.state.prefetch[0] & 0xFF) as u32;
                cpu.state.micro.scratch[3] = bit_num;
                prefetch_extension(cpu);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                let bit_num = cpu.state.micro.scratch[3];
                let val = cpu.state.d[reg_d];
                let res = execute_bchg(&mut cpu.state, bit_num, val, true);
                cpu.write_d_reg(reg_d, res, Size::Long);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            2 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            3 => {
                cpu.state.ir = cpu.state.prefetch[0];
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                cpu.state.micro.reset();
                StepResult::InstructionCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let phase = cpu.state.micro.scratch[2];
        if phase == 0 {
            let step = cpu.state.micro.micro_step;
            if step == 0 {
                let bit_num = (cpu.state.prefetch[0] & 0xFF) as u32;
                cpu.state.micro.scratch[3] = bit_num;
                prefetch_extension(cpu);
                return StepResult::StepCompleted;
            }

            let ea_step = step - 1;
            if ea_step == 0 {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
            }
            let val = match read_ea_operand(cpu, bus, ea_step, SIZE_BYTE, m) {
                Ok(v) => v,
                Err(res) => return res,
            };

            let bit_num = cpu.state.micro.scratch[3];
            let res = execute_bchg(&mut cpu.state, bit_num, val, false);
            cpu.state.micro.scratch[1] = res;
            cpu.initiate_prefetch();
            cpu.state.micro.scratch[2] = 1;
            StepResult::StepCompleted
        } else {
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            let addr = cpu.state.micro.scratch[0];
            let val = (cpu.state.micro.scratch[1] & 0xFF) as u16;
            let fc = data_fc(cpu);
            cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, BusAccessSize::Byte, fc));
            cpu.state.micro.mark_scratch_prefetch_retire();
            StepResult::StepCompleted
        }
    }
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_bchg_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_dn(cpu, bus)
}

#[inline(always)]
pub fn op_bchg_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bchg_imm(cpu, bus)
}

