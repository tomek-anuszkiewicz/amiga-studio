//! Cycle-exact linear M68000 logic instructions (AND, OR, EOR, NOT)
//!
//! Provides flat, branchless concrete instruction handlers eliminating
//! cascaded runtime branching in the hot instruction dispatch loop (Rule 2.6).

use super::linear_ea::*;
use super::logic;
use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// AND: <ea>, Dn
// ============================================================================

pub fn op_and_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_and_ea_to_dn(cpu, bus, s, m)
}

#[inline(always)]
fn exec_and_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
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
                    let res = logic::execute_and(&mut cpu.state, s_val, d_val, Size::Long);
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
            let res = logic::execute_and(&mut cpu.state, s_val, d_val, Size::Long);
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
        let res = logic::execute_and(&mut cpu.state, s_val, d_val, size);
        cpu.write_d_reg(reg_d, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// OR: <ea>, Dn
// ============================================================================

pub fn op_or_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_or_ea_to_dn(cpu, bus, s, m)
}

#[inline(always)]
fn exec_or_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
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
                    let res = logic::execute_or(&mut cpu.state, s_val, d_val, Size::Long);
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
            let res = logic::execute_or(&mut cpu.state, s_val, d_val, Size::Long);
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
        let res = logic::execute_or(&mut cpu.state, s_val, d_val, size);
        cpu.write_d_reg(reg_d, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// AND: Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_and_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_and_dn_to_ea(cpu, bus, s, m)
}

#[inline(always)]
fn exec_and_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
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
                    let res = logic::execute_and(&mut cpu.state, s_val, d_val, Size::Long);
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
            let res = logic::execute_and(&mut cpu.state, s_val, d_val, size);
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
            let res = logic::execute_and(&mut cpu.state, d_val, mem_val, size);
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

// ============================================================================
// OR: Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_or_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_or_dn_to_ea(cpu, bus, s, m)
}

#[inline(always)]
fn exec_or_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
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
                    let res = logic::execute_or(&mut cpu.state, s_val, d_val, Size::Long);
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
            let res = logic::execute_or(&mut cpu.state, s_val, d_val, size);
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
            let res = logic::execute_or(&mut cpu.state, d_val, mem_val, size);
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
                    let res = logic::execute_eor(&mut cpu.state, s_val, d_val, Size::Long);
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
            let res = logic::execute_eor(&mut cpu.state, s_val, d_val, size);
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
            let res = logic::execute_eor(&mut cpu.state, d_val, mem_val, size);
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
