//! Cycle-exact linear M68000 arithmetic instructions (ADD, SUB, ADDA, SUBA, ADDQ, SUBQ)
//!
//! Provides flat, branchless concrete instruction handlers eliminating
//! cascaded runtime branching in the hot instruction dispatch loop (Rule 2.6).

use super::arithmetic;
use super::linear_ea::*;
use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// ADD & SUB: <ea>, Dn
// ============================================================================

pub fn op_add_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_add_ea_to_dn(cpu, bus, s, m)
}

pub fn op_sub_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_sub_ea_to_dn(cpu, bus, s, m)
}

#[inline(always)]
fn exec_add_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    if s == SIZE_LONG {
        if m == EA_DN || m == EA_AN {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
                    let reg_s = (cpu.state.ir & 7) as usize;
                    let s_val = if m == EA_DN {
                        cpu.state.d[reg_s]
                    } else {
                        cpu.state.read_a(reg_s)
                    };
                    let d_val = cpu.state.d[reg_d];
                    let res = arithmetic::execute_add(&mut cpu.state, s_val, d_val, Size::Long, true);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };
            let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
            let d_val = cpu.state.d[reg_d];
            let res = arithmetic::execute_add(&mut cpu.state, s_val, d_val, Size::Long, true);
            cpu.write_d_reg(reg_d, res, Size::Long);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = arithmetic::execute_add(&mut cpu.state, s_val, d_val, size, true);
        cpu.write_d_reg(reg_d, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

#[inline(always)]
fn exec_sub_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    if s == SIZE_LONG {
        if m == EA_DN || m == EA_AN {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
                    let reg_s = (cpu.state.ir & 7) as usize;
                    let s_val = if m == EA_DN {
                        cpu.state.d[reg_s]
                    } else {
                        cpu.state.read_a(reg_s)
                    };
                    let d_val = cpu.state.d[reg_d];
                    let res = arithmetic::execute_sub(&mut cpu.state, s_val, d_val, Size::Long, true);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };
            let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
            let d_val = cpu.state.d[reg_d];
            let res = arithmetic::execute_sub(&mut cpu.state, s_val, d_val, Size::Long, true);
            cpu.write_d_reg(reg_d, res, Size::Long);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = arithmetic::execute_sub(&mut cpu.state, s_val, d_val, size, true);
        cpu.write_d_reg(reg_d, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// ADD & SUB: Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_add_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_add_dn_to_ea(cpu, bus, s, m)
}

pub fn op_sub_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_sub_dn_to_ea(cpu, bus, s, m)
}

#[inline(always)]
fn exec_add_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    let phase = cpu.state.micro.scratch[2];
    if phase == 0 {
        let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = arithmetic::execute_add(&mut cpu.state, d_val, mem_val, size, true);
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

#[inline(always)]
fn exec_sub_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    let phase = cpu.state.micro.scratch[2];
    if phase == 0 {
        let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = arithmetic::execute_sub(&mut cpu.state, d_val, mem_val, size, true);
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

// ============================================================================
// ADDA & SUBA: <ea>, An
// ============================================================================

pub fn op_adda(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = if ((ir >> 8) & 1) != 0 { SIZE_LONG } else { SIZE_WORD };
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_adda(cpu, bus, s, m)
}

pub fn op_suba(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = if ((ir >> 8) & 1) != 0 { SIZE_LONG } else { SIZE_WORD };
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_suba(cpu, bus, s, m)
}

#[inline(always)]
fn exec_adda(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    if m == EA_DN || m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
                let reg_s = (cpu.state.ir & 7) as usize;
                let raw_val = if m == EA_DN {
                    cpu.state.d[reg_s]
                } else {
                    cpu.state.read_a(reg_s)
                };
                let s_val = if s == SIZE_WORD {
                    (raw_val as i16 as i32) as u32
                } else {
                    raw_val
                };
                let a_val = cpu.state.read_a(reg_a);
                let res = a_val.wrapping_add(s_val);
                cpu.state.write_a(reg_a, res);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let raw_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
        let s_val = if s == SIZE_WORD {
            (raw_val as i16 as i32) as u32
        } else {
            raw_val
        };
        let a_val = cpu.state.read_a(reg_a);
        let res = a_val.wrapping_add(s_val);
        cpu.state.write_a(reg_a, res);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

#[inline(always)]
fn exec_suba(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    if m == EA_DN || m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
                let reg_s = (cpu.state.ir & 7) as usize;
                let raw_val = if m == EA_DN {
                    cpu.state.d[reg_s]
                } else {
                    cpu.state.read_a(reg_s)
                };
                let s_val = if s == SIZE_WORD {
                    (raw_val as i16 as i32) as u32
                } else {
                    raw_val
                };
                let a_val = cpu.state.read_a(reg_a);
                let res = a_val.wrapping_sub(s_val);
                cpu.state.write_a(reg_a, res);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let raw_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
        let s_val = if s == SIZE_WORD {
            (raw_val as i16 as i32) as u32
        } else {
            raw_val
        };
        let a_val = cpu.state.read_a(reg_a);
        let res = a_val.wrapping_sub(s_val);
        cpu.state.write_a(reg_a, res);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// ADDQ & SUBQ: #<data>, <ea>
// ============================================================================

pub fn op_addq(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_addq(cpu, bus, s, m)
}

pub fn op_subq(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_subq(cpu, bus, s, m)
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
                    let res = arithmetic::execute_add(&mut cpu.state, imm, d, Size::Long, true);
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
            let res = arithmetic::execute_add(&mut cpu.state, imm, d, size, true);
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
            let res = arithmetic::execute_add(&mut cpu.state, imm, mem_val, size, true);
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

#[inline(always)]
fn exec_subq(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
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
                let res = a.wrapping_sub(imm);
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
                    let res = arithmetic::execute_sub(&mut cpu.state, imm, d, Size::Long, true);
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
            let res = arithmetic::execute_sub(&mut cpu.state, imm, d, size, true);
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
            let res = arithmetic::execute_sub(&mut cpu.state, imm, mem_val, size, true);
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
