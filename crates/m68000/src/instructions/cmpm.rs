//! CMPM (Compare Memory) instruction handlers
//!
//! Compares memory operands via postincrement: `CMPM (Ay)+, (Ax)+`.
//! Evaluates ((Ax) - (Ay)) and updates N, Z, V, and C flags.
//! Neither memory location is modified. Extend (X) flag is unaffected.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::cmp::execute_cmp;
use crate::instructions::ea::{
    bus_size_from_const, data_fc, size_from_const, trigger_address_error, SIZE_BYTE, SIZE_LONG,
    SIZE_WORD,
};
use memory_bus::{BusCycle, MemoryBus};

/// Execution handler for `CMPM (Ay)+, (Ax)+`
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
                execute_cmp(&mut cpu.state, val_y, val_x, Size::Long);
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
                execute_cmp(&mut cpu.state, val_y, val_x, size);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_cmpm_b_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}

#[inline(always)]
pub fn op_cmpm_l_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}

#[inline(always)]
pub fn op_cmpm_w_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpm(cpu, bus)
}

