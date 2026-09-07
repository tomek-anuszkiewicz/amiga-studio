//! Cycle-exact linear M68000 comparison instructions (CMP, CMPA, CMPM, TST)
//!
//! Provides flat, branchless concrete instruction handlers eliminating
//! cascaded runtime branching in the hot instruction dispatch loop (Rule 2.6).

use super::arithmetic;
use super::linear_ea::*;
use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use memory_bus::{BusCycle, MemoryBus};

// ============================================================================
// CMP & CMPA: <ea>, Dn / An
// ============================================================================

pub fn op_cmp_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let size = size_from_const(s);
    if s == SIZE_LONG && (m == EA_DN || m == EA_AN) {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(2);
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
                arithmetic::execute_cmp(&mut cpu.state, s_val, d_val, Size::Long);
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
        arithmetic::execute_cmp(&mut cpu.state, s_val, d_val, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

pub fn op_cmpa(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = if ((ir >> 8) & 1) != 0 { SIZE_LONG } else { SIZE_WORD };
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    if m == EA_DN || m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(2);
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
                arithmetic::execute_cmp(&mut cpu.state, s_val, a_val, Size::Long);
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
        arithmetic::execute_cmp(&mut cpu.state, s_val, a_val, Size::Long);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// CMPM: (Ay)+, (Ax)+
// ============================================================================

pub fn op_cmpm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    if s == SIZE_LONG {
        match cpu.state.micro.micro_step {
            0 => {
                let reg_y = (cpu.state.ir & 7) as usize;
                let addr_y = cpu.state.read_a(reg_y);
                cpu.state.write_a(reg_y, addr_y.wrapping_add(4));
                if (addr_y & 1) != 0 {
                    return trigger_address_error(cpu, addr_y, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_y;
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, bus_size, fc));
                StepResult::StepCompleted
            }
            1 => {
                let hi_y = cpu.state.micro.last_read;
                cpu.state.micro.scratch[1] = (hi_y as u32) << 16;
                let addr_y2 = cpu.state.micro.scratch[0].wrapping_add(2);
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y2, bus_size, fc));
                StepResult::StepCompleted
            }
            2 => {
                let lo_y = cpu.state.micro.last_read;
                cpu.state.micro.scratch[1] |= lo_y as u32;
                let reg_x = ((cpu.state.ir >> 9) & 7) as usize;
                let addr_x = cpu.state.read_a(reg_x);
                cpu.state.write_a(reg_x, addr_x.wrapping_add(4));
                if (addr_x & 1) != 0 {
                    return trigger_address_error(cpu, addr_x, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_x;
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, bus_size, fc));
                StepResult::StepCompleted
            }
            3 => {
                let hi_x = cpu.state.micro.last_read;
                let addr_x2 = cpu.state.micro.scratch[0].wrapping_add(2);
                cpu.state.micro.scratch[0] = (hi_x as u32) << 16;
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x2, bus_size, fc));
                StepResult::StepCompleted
            }
            4 => {
                let lo_x = cpu.state.micro.last_read;
                let val_x = cpu.state.micro.scratch[0] | (lo_x as u32);
                let val_y = cpu.state.micro.scratch[1];
                arithmetic::execute_cmp(&mut cpu.state, val_y, val_x, Size::Long);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        match cpu.state.micro.micro_step {
            0 => {
                let reg_y = (cpu.state.ir & 7) as usize;
                let addr_y = cpu.state.read_a(reg_y);
                let inc = if reg_y == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                cpu.state.write_a(reg_y, addr_y.wrapping_add(inc));
                if s != SIZE_BYTE && (addr_y & 1) != 0 {
                    return trigger_address_error(cpu, addr_y, true, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, bus_size, fc));
                StepResult::StepCompleted
            }
            1 => {
                let val_y = match s {
                    SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                    _ => cpu.state.micro.last_read as u32,
                };
                cpu.state.micro.scratch[1] = val_y;
                let reg_x = ((cpu.state.ir >> 9) & 7) as usize;
                let addr_x = cpu.state.read_a(reg_x);
                let inc = if reg_x == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                cpu.state.write_a(reg_x, addr_x.wrapping_add(inc));
                if s != SIZE_BYTE && (addr_x & 1) != 0 {
                    return trigger_address_error(cpu, addr_x, true, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, bus_size, fc));
                StepResult::StepCompleted
            }
            2 => {
                let val_y = cpu.state.micro.scratch[1];
                let val_x = match s {
                    SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                    _ => cpu.state.micro.last_read as u32,
                };
                arithmetic::execute_cmp(&mut cpu.state, val_y, val_x, size);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

// ============================================================================
// TST: <ea>
// ============================================================================

pub fn op_tst(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let size = size_from_const(s);
    let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
        Ok(v) => v,
        Err(res) => return res,
    };
    arithmetic::execute_tst(&mut cpu.state, val, size);
    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}
