//! Specialized Micro-Step Execution Handlers for M68000 CPU

use crate::core::{Cpu, StepResult};
use crate::micro::types::MicroStep;
use memory_bus::{CckPhase, MemoryBus};

impl Cpu {
    pub(crate) fn execute_alu_step(&mut self, step: MicroStep) -> Option<StepResult> {
        let clocks = if self.state.micro.internal_clocks > 0 {
            self.state.micro.internal_clocks
        } else {
            step.base_clocks as u16
        };
        if clocks > 0 {
            self.state.micro.internal_clocks = clocks.saturating_sub(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if self.state.micro.internal_clocks == 0 {
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
            }
            Some(StepResult::StepCompleted)
        } else {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
            None
        }
    }

    pub fn step_alu(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let step = self.state.micro.current_steps[self.state.micro.micro_step as usize];
        self.execute_alu_step(step)
    }

    pub fn step_branch_eval(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        None
    }

    pub fn step_bus_read_byte(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        let res = self.step_read_byte_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_read_word(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let res = self.step_read_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_read_long_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let res = self.step_read_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.scratch[0] = (self.state.micro.last_read as u32) << 16;
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_read_long_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let res = self.step_read_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.scratch[1] =
                self.state.micro.scratch[0] | (self.state.micro.last_read as u32);
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_write_byte(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        let val = (self.state.micro.write_buffer & 0xFF) as u8;
        let res = self.step_write_byte_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_write_word(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_write_long_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_write_long_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_write_word_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.state.ir = self.state.prefetch[0];
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            return Some(self.retire_scratch_prefetch());
        }
        Some(res)
    }

    pub fn step_bus_write_byte_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        let val = (self.state.micro.write_buffer & 0xFF) as u8;
        let res = self.step_write_byte_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            return Some(self.retire_scratch_prefetch());
        }
        Some(res)
    }

    pub fn step_bus_write_long_low_and_retire(
        &mut self,
        bus: &mut MemoryBus,
    ) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            return Some(self.retire_scratch_prefetch());
        }
        Some(res)
    }

    pub fn step_bus_write_long_high_and_retire(
        &mut self,
        bus: &mut MemoryBus,
    ) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, addr, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            return Some(self.retire_scratch_prefetch());
        }
        Some(res)
    }

    pub fn step_bus_pop_stack(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, true, false, bus));
        }
        let res = self.step_read_word_at(bus, sp);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.write_a(7, sp.wrapping_add(2));
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_pop_stack_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, true, false, bus));
        }
        let res = self.step_read_word_at(bus, sp);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.write_a(7, sp.wrapping_add(2));
            self.state.micro.scratch[0] = (self.state.micro.last_read as u32) << 16;
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_pop_stack_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, true, false, bus));
        }
        let res = self.step_read_word_at(bus, sp);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.write_a(7, sp.wrapping_add(2));
            self.state.micro.ea_addr =
                self.state.micro.scratch[0] | (self.state.micro.last_read as u32);
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_push_stack_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = if self.state.micro.phase == CckPhase::Cck1 {
            let s = self.state.read_a(7).wrapping_sub(4);
            self.state.write_a(7, s);
            s
        } else {
            self.state.read_a(7)
        };
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, false, false, bus));
        }
        let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, sp, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_push_stack_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp_low = self.state.read_a(7).wrapping_add(2);
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, sp_low, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_push_stack_low_and_retire(
        &mut self,
        bus: &mut MemoryBus,
    ) -> Option<StepResult> {
        let sp_low = self.state.read_a(7).wrapping_add(2);
        let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
        let res = self.step_write_word_at(bus, sp_low, val);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            return Some(self.retire_scratch_prefetch());
        }
        Some(res)
    }

    pub fn step_fetch_extension(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.pc;
        let res = self.step_read_prog_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.prefetch[0] = self.state.micro.last_read;
            self.state.pc = self.state.pc.wrapping_add(2);
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_bus_prefetch_to_scratch(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.pc;
        let res = self.step_read_prog_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.scratch_prefetch = self.state.micro.last_read;
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_prefetch_next_opcode_and_retire(
        &mut self,
        bus: &mut MemoryBus,
    ) -> Option<StepResult> {
        let addr = self.state.pc;
        let res = self.step_read_prog_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            let word = self.state.micro.last_read;
            return Some(self.retire_standard(word));
        }
        Some(res)
    }

    pub fn step_bus_read_target_opcode(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, true, bus));
        }
        let res = self.step_read_prog_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            self.state.micro.scratch_prefetch = self.state.micro.last_read;
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        }
        Some(res)
    }

    pub fn step_prefetch_target_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        let target = self.state.micro.ea_addr;
        let new_ir = self.state.micro.scratch_prefetch;
        let res = self.step_read_prog_word_at(bus, addr);
        if res == StepResult::StepCompleted && self.state.micro.phase == CckPhase::Cck1 {
            let target_prefetch = self.state.micro.last_read;
            return Some(self.retire_target_refill(target, target_prefetch, new_ir));
        }
        Some(res)
    }
}
