//! Cycle-exact linear M68000 bit manipulation instructions (BTST, BCHG, BCLR, BSET)
//!
//! Provides flat, branchless concrete instruction handlers eliminating
//! cascaded runtime branching in the hot instruction dispatch loop (Rule 2.6).

use super::bits;
use super::linear_ea::*;
use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// Dynamic Bit Operations: Dn, <ea>
// ============================================================================

pub fn op_btst_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_btst_dn(cpu, bus, m)
}

pub fn op_bchg_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bchg_dn(cpu, bus, m)
}

pub fn op_bclr_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bclr_dn(cpu, bus, m)
}

pub fn op_bset_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bset_dn(cpu, bus, m)
}

#[inline(always)]
fn exec_btst_dn(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d[reg_bit];

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d[reg_d];
                bits::execute_btst(&mut cpu.state, bit_num, val, true);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            1 => {
                cpu.record_internal_clocks(2);
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
        let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, SIZE_BYTE, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        bits::execute_btst(&mut cpu.state, bit_num, val, false);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

#[inline(always)]
fn exec_bchg_dn(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d[reg_bit];

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d[reg_d];
                let res = bits::execute_bchg(&mut cpu.state, bit_num, val, true);
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
            let res = bits::execute_bchg(&mut cpu.state, bit_num, val, false);
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

#[inline(always)]
fn exec_bclr_dn(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d[reg_bit];

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d[reg_d];
                let res = bits::execute_bclr(&mut cpu.state, bit_num, val, true);
                cpu.write_d_reg(reg_d, res, Size::Long);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            1 => {
                cpu.record_internal_clocks(6);
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
            let res = bits::execute_bclr(&mut cpu.state, bit_num, val, false);
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

#[inline(always)]
fn exec_bset_dn(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d[reg_bit];

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d[reg_d];
                let res = bits::execute_bset(&mut cpu.state, bit_num, val, true);
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
            let res = bits::execute_bset(&mut cpu.state, bit_num, val, false);
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

// ============================================================================
// Static Bit Operations: #<data>, <ea>
// ============================================================================

pub fn op_btst_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_btst_imm(cpu, bus, m)
}

pub fn op_bchg_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bchg_imm(cpu, bus, m)
}

pub fn op_bclr_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bclr_imm(cpu, bus, m)
}

pub fn op_bset_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    exec_bset_imm(cpu, bus, m)
}

#[inline(always)]
fn exec_btst_imm(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
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
                bits::execute_btst(&mut cpu.state, bit_num, val, true);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            2 => {
                cpu.record_internal_clocks(2);
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
        bits::execute_btst(&mut cpu.state, bit_num, val, false);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

#[inline(always)]
fn exec_bchg_imm(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
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
                let res = bits::execute_bchg(&mut cpu.state, bit_num, val, true);
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
            let res = bits::execute_bchg(&mut cpu.state, bit_num, val, false);
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

#[inline(always)]
fn exec_bclr_imm(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
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
                let res = bits::execute_bclr(&mut cpu.state, bit_num, val, true);
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
            let res = bits::execute_bclr(&mut cpu.state, bit_num, val, false);
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

#[inline(always)]
fn exec_bset_imm(cpu: &mut Cpu, bus: &mut MemoryBus, m: u8) -> StepResult {
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
                let res = bits::execute_bset(&mut cpu.state, bit_num, val, true);
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
            let res = bits::execute_bset(&mut cpu.state, bit_num, val, false);
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
