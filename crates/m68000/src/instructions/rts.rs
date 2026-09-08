//! RTS (Return from Subroutine) instruction handler
//!
//! Pops a 32-bit program counter from the stack and refills the instruction prefetch queue.
//! Execution time: 16 CPU clocks (8 CCKs).

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::engine::trigger_address_error_step;
use crate::micro::types::MicroStep;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Cycle-exact micro-step sequence for `RTS` (16 CPU clocks / 8 CCKs)
pub static STEPS_RTS: [MicroStep; 4] = [
    common::POP_STACK_HIGH,
    common::POP_STACK_LOW,
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

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

/// Execution handler for `RTS` (16 CPU clocks / 8 CCKs)
pub fn op_rts(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let fc_d = data_fc(cpu);
    let fc_p = prog_fc(cpu);

    match cpu.state.micro.micro_step {
        0 => {
            let sp = cpu.state.read_a(7);
            if (sp & 1) != 0 {
                return trigger_address_error_step(cpu, sp, true, false, bus);
            }
            cpu.initiate_bus_cycle(BusCycle::new_read(sp, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        1 => {
            let hi = (cpu.state.micro.last_read as u32) << 16;
            cpu.state.micro.scratch[0] = hi;
            let sp = cpu.state.read_a(7).wrapping_add(2);
            cpu.initiate_bus_cycle(BusCycle::new_read(sp, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        2 => {
            let lo = cpu.state.micro.last_read as u32;
            let target = cpu.state.micro.scratch[0] | lo;
            let sp = cpu.state.read_a(7).wrapping_add(4);
            cpu.state.write_a(7, sp);
            if (target & 1) != 0 {
                return trigger_address_error_step(cpu, target, true, true, bus);
            }
            cpu.state.micro.scratch[0] = target;
            cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
            StepResult::StepCompleted
        }
        3 => {
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
