//! BRA (Branch Always) instruction handler
//!
//! Unconditional relative branch with 8-bit short or 16-bit word displacement.
//! Execution time: 10 CPU clocks (2 internal idle clocks + 2 bus prefetch cycles).

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

/// Specialized execution handler for unconditional `BRA`
pub fn op_bra(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let d8 = (opcode & 0x00FF) as i8;
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);

    if d8 != 0 {
        // --- 8-bit Short Displacement Branch (10 CPU clocks) ---
        let target = base_pc.wrapping_add(d8 as i32 as u32);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.internal_clocks = 2;
                StepResult::StepCompleted
            }
            1 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
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
        // --- 16-bit Word Displacement Branch (10 CPU clocks) ---
        let disp = cpu.state.prefetch[0] as i16 as i32;
        let target = base_pc.wrapping_add(disp as u32);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.internal_clocks = 2;
                StepResult::StepCompleted
            }
            1 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
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
