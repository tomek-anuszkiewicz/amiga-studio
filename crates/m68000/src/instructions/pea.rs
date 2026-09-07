//! PEA (Push Effective Address) instruction handler

use crate::addressing::{AddressingMode, Size};
use crate::core::{Cpu, StepResult};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

pub fn op_pea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let mode = ((opcode >> 3) & 0x07) as u8;
    let reg = (opcode & 0x07) as u8;

    // Archetype 6: PEA (An) (Stack Push - Multi-Cycle, 12 clocks / 6 CCKs)
    if mode == 2 {
        let fc_data = if cpu.state.is_supervisor() {
            memory_bus::function_code::SUPERVISOR_DATA
        } else {
            memory_bus::function_code::USER_DATA
        };

        match cpu.state.micro.micro_step {
            0 => {
                let addr = cpu.state.read_a(reg as usize);
                cpu.state.micro.scratch[0] = addr;
                cpu.initiate_prefetch();
                return StepResult::StepCompleted;
            }
            1 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.read_a(7).wrapping_sub(4);
                cpu.state.write_a(7, sp);
                if (sp & 1) != 0 {
                    cpu.handle_address_error(sp, false, bus);
                    return StepResult::InstructionCompleted;
                }
                let hi = ((cpu.state.micro.scratch[0] >> 16) & 0xFFFF) as u16;
                let cycle = BusCycle::new_write(
                    sp,
                    hi,
                    BusAccessSize::Word,
                    fc_data,
                );
                cpu.initiate_bus_cycle(cycle);
                return StepResult::StepCompleted;
            }
            2 => {
                let sp_low = cpu.state.read_a(7).wrapping_add(2);
                let lo = (cpu.state.micro.scratch[0] & 0xFFFF) as u16;
                let cycle = BusCycle::new_write(
                    sp_low,
                    lo,
                    BusAccessSize::Word,
                    fc_data,
                );
                cpu.initiate_bus_cycle(cycle);
                cpu.state.micro.mark_scratch_prefetch_retire();
                return StepResult::StepCompleted;
            }
            _ => unreachable!(),
        }
    }

    // Fallback for other addressing modes (absolute, PC-relative, etc.)
    let pc = cpu.state.pc.wrapping_sub(2);
    let mut ext_reader = || cpu.consume_extension_word(bus);
    match AddressingMode::decode(mode, reg, Size::Long, pc, &mut ext_reader) {
        Ok(ea) => match ea.resolve_address_unaligned(&mut cpu.state) {
            Ok(addr) => {
                let sp = cpu.state.read_a(7).wrapping_sub(4);
                cpu.state.write_a(7, sp);
                if (sp & 1) != 0 {
                    cpu.handle_address_error(sp, false, bus);
                    return StepResult::InstructionCompleted;
                }
                bus.write_word_debug(sp, (addr >> 16) as u16);
                bus.write_word_debug(sp.wrapping_add(2), (addr & 0xFFFF) as u16);
                cpu.retire_instruction(bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        },
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}
