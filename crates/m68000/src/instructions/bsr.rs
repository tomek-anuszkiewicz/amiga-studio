//! BSR (Branch to Subroutine) instruction handler
//!
//! Subroutine branch pushing the return PC onto the stack.
//! Supports 8-bit short and 16-bit word displacements.
//! Execution time: 18 CPU clocks (2 internal + 2 stack writes + 2 prefetch).

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::trigger_address_error;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

#[inline(always)]
fn prog_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_PROGRAM
    } else {
        memory_bus::function_code::USER_PROGRAM
    }
}

#[inline(always)]
fn data_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_DATA
    } else {
        memory_bus::function_code::USER_DATA
    }
}

/// Specialized execution handler for `BSR`
pub fn op_bsr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let d8 = (opcode & 0x00FF) as i8;
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);
    let fc_d = data_fc(cpu);

    if d8 != 0 {
        // --- 8-bit Short Displacement BSR ---
        let target = base_pc.wrapping_add(d8 as i32 as u32);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let return_pc = base_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                StepResult::StepCompleted
            }
            1 => {
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            2 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            4 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    target.wrapping_add(2),
                    BusAccessSize::Word,
                    fc_p,
                ));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        // --- 16-bit Word Displacement BSR ---
        let disp = cpu.state.prefetch[0] as i16 as i32;
        let target = base_pc.wrapping_add(disp as u32);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let return_pc = base_pc.wrapping_add(2);
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                StepResult::StepCompleted
            }
            1 => {
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            2 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            4 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    target.wrapping_add(2),
                    BusAccessSize::Word,
                    fc_p,
                ));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}
