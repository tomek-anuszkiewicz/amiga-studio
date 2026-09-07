//! M68000 ORI Instruction (`ORI #<imm>, <ea>`, `ORI #<imm>, CCR`, `ORI #<imm>, SR`)
//!
//! Performs bitwise OR of an immediate operand with a destination data register,
//! memory effective address, CCR, or SR.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::*;
use crate::instructions::or::execute_or;
use crate::instructions::system;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

pub fn op_ori(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_ori(cpu, bus, s, m)
}

#[inline(always)]
fn exec_ori(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);

    if m == EA_DN {
        if s == SIZE_LONG {
            match cpu.state.micro.micro_step {
                0 => {
                    let hi = cpu.state.prefetch[0];
                    cpu.state.micro.scratch[0] = (hi as u32) << 16;
                    prefetch_extension(cpu);
                    StepResult::StepCompleted
                }
                1 => {
                    let lo = cpu.state.micro.last_read;
                    cpu.state.micro.scratch[0] |= lo as u32;
                    prefetch_extension(cpu);
                    StepResult::StepCompleted
                }
                2 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                3 => {
                    let imm = cpu.state.micro.scratch[0];
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let d = cpu.state.d[reg_d];
                    let res = execute_or(&mut cpu.state, imm, d, Size::Long);
                    cpu.write_d_reg(reg_d, res, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            match cpu.state.micro.micro_step {
                0 => {
                    let imm = match s {
                        SIZE_BYTE => (cpu.state.prefetch[0] & 0xFF) as u32,
                        _ => cpu.state.prefetch[0] as u32,
                    };
                    cpu.state.micro.scratch[0] = imm;
                    prefetch_extension(cpu);
                    StepResult::StepCompleted
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let imm = cpu.state.micro.scratch[0];
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let d = cpu.state.d[reg_d];
                    let res = execute_or(&mut cpu.state, imm, d, size);
                    cpu.write_d_reg(reg_d, res, size);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    } else {
        // Memory destination: Class 0 Read-Modify-Write
        let bus_size = bus_size_from_const(s);
        let fc = data_fc(cpu);
        let phase = cpu.state.micro.scratch[2];

        if s == SIZE_LONG {
            if phase == 0 {
                let step = cpu.state.micro.micro_step;
                if step == 0 {
                    let hi = cpu.state.prefetch[0];
                    cpu.state.micro.scratch[3] = (hi as u32) << 16;
                    prefetch_extension(cpu);
                    return StepResult::StepCompleted;
                } else if step == 1 {
                    let lo = cpu.state.micro.last_read;
                    cpu.state.micro.scratch[3] |= lo as u32;
                    prefetch_extension(cpu);
                    return StepResult::StepCompleted;
                }

                let ea_step = step - 2;
                if ea_step == 0 {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                }
                let mem_val = match read_ea_operand(cpu, bus, ea_step, s, m) {
                    Ok(v) => v,
                    Err(res) => return res,
                };

                let imm = cpu.state.micro.scratch[3];
                let res = execute_or(&mut cpu.state, imm, mem_val, Size::Long);
                cpu.state.micro.scratch[1] = res;
                cpu.initiate_prefetch();
                cpu.state.micro.scratch[2] = 1;
                StepResult::StepCompleted
            } else if phase == 1 {
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
            // Byte / Word memory destination
            if phase == 0 {
                let step = cpu.state.micro.micro_step;
                if step == 0 {
                    let imm = match s {
                        SIZE_BYTE => (cpu.state.prefetch[0] & 0xFF) as u32,
                        _ => cpu.state.prefetch[0] as u32,
                    };
                    cpu.state.micro.scratch[3] = imm;
                    prefetch_extension(cpu);
                    return StepResult::StepCompleted;
                }

                let ea_step = step - 1;
                if ea_step == 0 {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                }
                let mem_val = match read_ea_operand(cpu, bus, ea_step, s, m) {
                    Ok(v) => v,
                    Err(res) => return res,
                };

                let imm = cpu.state.micro.scratch[3];
                let res = execute_or(&mut cpu.state, imm, mem_val, size);
                cpu.state.micro.scratch[1] = res;
                cpu.initiate_prefetch();
                cpu.state.micro.scratch[2] = 1;
                StepResult::StepCompleted
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
}

/// ORI to CCR (Opcode 0x003C)
pub fn op_ori_to_ccr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let imm = cpu.consume_extension_word(bus) & 0x1F;
    cpu.state.sr = (cpu.state.sr & !0x1F) | ((cpu.state.sr | imm) & 0x1F);
    cpu.retire_instruction(bus);
    StepResult::InstructionCompleted
}

/// ORI to SR (Opcode 0x007C, privileged)
pub fn op_ori_to_sr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    if !cpu.state.is_supervisor() {
        let ret_pc = cpu.state.instruction_pc;
        system::push_standard_exception(
            &mut cpu.state,
            system::VECTOR_PRIVILEGE_VIOLATION,
            ret_pc,
            bus,
        );
        cpu.reload_pc_and_prefetch(cpu.state.pc, bus);
        return StepResult::InstructionCompleted;
    }
    let imm = cpu.consume_extension_word(bus);
    let new_sr = cpu.state.sr | imm;
    cpu.state.set_sr(new_sr);
    cpu.retire_instruction(bus);
    StepResult::InstructionCompleted
}
