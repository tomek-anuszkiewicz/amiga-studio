//! TRAP instruction handler
//!
//! Initiates exception processing for TRAP #0..15 (vectors 32..47).
//! Pushes return PC and SR to the supervisor stack and jumps through vector.
//! Execution time: 34 CPU clocks (17 CCKs).

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::trigger_address_error;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Execution handler for `TRAP #<vector>` (34 CPU clocks / 17 CCKs)
pub fn op_trap(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let vec = (opcode & 0x000F) as u32;
    let vector_addr = 0x0000_0080 + vec * 4;
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_d = memory_bus::function_code::SUPERVISOR_DATA;
    let fc_p = memory_bus::function_code::SUPERVISOR_PROGRAM;

    match cpu.state.micro.micro_step {
        0 => {
            // 4 internal clocks
            cpu.state.micro.internal_clocks = 4;
            let return_pc = base_pc;
            let old_sr = cpu.state.sr;
            // Switch to supervisor mode (S=1, T=0)
            cpu.state.set_supervisor(true);
            cpu.state.sr &= !0x8000;
            cpu.state.micro.scratch[0] = return_pc;
            cpu.state.micro.scratch[1] = old_sr as u32;
            cpu.state.micro.scratch[2] = vector_addr;
            StepResult::StepCompleted
        }
        1 => {
            // Write return PC low word to SP - 2
            let sp = cpu.state.read_a(7);
            let lo = (cpu.state.micro.scratch[0] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(
                sp.wrapping_sub(2),
                lo,
                BusAccessSize::Word,
                fc_d,
            ));
            StepResult::StepCompleted
        }
        2 => {
            // Write old SR to SP - 6
            let sp = cpu.state.read_a(7);
            let sr = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(
                sp.wrapping_sub(6),
                sr,
                BusAccessSize::Word,
                fc_d,
            ));
            StepResult::StepCompleted
        }
        3 => {
            // Write return PC high word to SP - 4 and update SP
            let sp = cpu.state.read_a(7);
            let hi = ((cpu.state.micro.scratch[0] >> 16) & 0xFFFF) as u16;
            cpu.state.write_a(7, sp.wrapping_sub(6));
            cpu.initiate_bus_cycle(BusCycle::new_write(
                sp.wrapping_sub(4),
                hi,
                BusAccessSize::Word,
                fc_d,
            ));
            StepResult::StepCompleted
        }
        4 => {
            // Read vector high word from vector_addr
            let vec_addr = cpu.state.micro.scratch[2];
            cpu.initiate_bus_cycle(BusCycle::new_read(vec_addr, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        5 => {
            // Read vector low word from vector_addr + 2
            let hi = (cpu.state.micro.last_read as u32) << 16;
            cpu.state.micro.scratch[0] = hi;
            let vec_addr = cpu.state.micro.scratch[2];
            cpu.initiate_bus_cycle(BusCycle::new_read(
                vec_addr.wrapping_add(2),
                BusAccessSize::Word,
                fc_d,
            ));
            StepResult::StepCompleted
        }
        6 => {
            // Latch vector target and read target opcode
            let lo = cpu.state.micro.last_read as u32;
            let target = (cpu.state.micro.scratch[0] | lo) & 0x00FF_FFFF;
            if (target & 1) != 0 {
                return trigger_address_error(cpu, target, true, true, bus);
            }
            cpu.state.micro.scratch[0] = target;
            cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
            StepResult::StepCompleted
        }
        7 => {
            // 2 internal clocks
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            cpu.state.micro.internal_clocks = 2;
            StepResult::StepCompleted
        }
        8 => {
            // Read target + 2 word and refill prefetch pipeline
            let target = cpu.state.micro.scratch[0];
            let new_ir = cpu.state.micro.scratch_prefetch;
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
