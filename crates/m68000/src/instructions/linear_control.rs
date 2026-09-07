//! Linearized cycle-exact Control Flow instruction implementations
//!
//! Covers:
//! - `NOP` (4 clocks / 2 CCKs)
//! - `BRA` and `Bcc` (16 conditions, short 8-bit & word 16-bit displacements, taken 10 clocks, untaken 8/12 clocks)
//! - `BSR` (18 clocks for short and word)
//! - `JMP` (Control addressing modes: `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`)
//! - `JSR` (Control addressing modes, 16–22 clocks)
//! - `RTS` (16 clocks)
//! - `TRAP` (34 clocks)

use crate::core::{Cpu, StepResult};
use crate::instructions::linear_ea::{
    decode_ea_index, trigger_address_error, EA_AI, EA_AL, EA_AW, EA_DI, EA_DIPC, EA_IX, EA_IXPC,
};
use memory_bus::{self, BusAccessSize, BusCycle, MemoryBus};

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

// ============================================================================
// 1. NOP
// ============================================================================

pub fn op_nop(cpu: &mut Cpu, _bus: &mut MemoryBus) -> StepResult {
    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}

// ============================================================================
// 2. BRA, BSR, and Bcc
// ============================================================================

/// Cycle-exact branch handler parameterized by condition code (0..15)
pub fn op_bra_bcc(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let cond = ((opcode >> 8) & 0x0F) as u8;
    let d8 = (opcode & 0x00FF) as i8;
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);
    let fc_d = data_fc(cpu);

    if d8 != 0 {
        // --- 8-bit Short Displacement Branch ---
        let target = base_pc.wrapping_add(d8 as i32 as u32);
        let taken = if cond == 0 {
            true
        } else if cond == 1 {
            true // BSR
        } else {
            cpu.state.eval_condition(cond)
        };

        if cond == 1 {
            // BSR Short: 18 CPU clocks (2 internal + 2 stack writes + 2 prefetch)
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
                    cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                    cpu.state.micro.mark_target_refill_retire(target, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else if taken {
            // Taken Bcc / BRA Short: 10 CPU clocks (2 internal + 2 prefetch)
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
                    cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                    cpu.state.micro.mark_target_refill_retire(target, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            // Untaken Bcc Short: 8 CPU clocks (4 internal + 4 prefetch)
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.internal_clocks = 4;
                    StepResult::StepCompleted
                }
                1 => {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    } else {
        // --- 16-bit Word Displacement Branch (d8 == 0) ---
        let disp = cpu.state.prefetch[0] as i16 as i32;
        let target = base_pc.wrapping_add(disp as u32);
        let taken = if cond == 0 {
            true
        } else if cond == 1 {
            true // BSR
        } else {
            cpu.state.eval_condition(cond)
        };

        if cond == 1 {
            // BSR Word: 18 CPU clocks (2 internal + 2 stack writes + 2 prefetch)
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
                    cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                    cpu.state.micro.mark_target_refill_retire(target, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else if taken {
            // Taken Bcc / BRA Word: 10 CPU clocks (2 internal + 2 prefetch)
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
                    cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                    cpu.state.micro.mark_target_refill_retire(target, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            // Untaken Bcc Word: 12 CPU clocks (4 internal + 2 prefetch)
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.internal_clocks = 4;
                    StepResult::StepCompleted
                }
                1 => {
                    let next_pc = base_pc.wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_read(next_pc, BusAccessSize::Word, fc_p));
                    StepResult::StepCompleted
                }
                2 => {
                    let new_ir = cpu.state.micro.last_read;
                    let next_pc = base_pc.wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_read(next_pc.wrapping_add(2), BusAccessSize::Word, fc_p));
                    cpu.state.micro.mark_target_refill_retire(next_pc, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    }
}

// ============================================================================
// 3. JMP
// ============================================================================

/// Calculates target address for control addressing modes
#[inline(always)]
fn resolve_control_target(cpu: &Cpu, mode: u8, reg: usize, base_pc: u32) -> u32 {
    match mode {
        EA_AI => cpu.state.read_a(reg),
        EA_DI => {
            let disp = cpu.state.prefetch[0] as i16 as i32;
            cpu.state.read_a(reg).wrapping_add(disp as u32)
        }
        EA_IX => {
            let ext = cpu.state.prefetch[0];
            let base = cpu.state.read_a(reg);
            let disp = (ext & 0xFF) as i8 as i32;
            let idx_reg = ((ext >> 12) & 7) as usize;
            let is_a = (ext & 0x8000) != 0;
            let is_long = (ext & 0x0800) != 0;
            let idx_val = if is_a { cpu.state.read_a(idx_reg) } else { cpu.state.d[idx_reg] };
            let idx_ext = if is_long { idx_val as i32 } else { (idx_val as i16) as i32 };
            base.wrapping_add(disp as u32).wrapping_add(idx_ext as u32)
        }
        EA_AW => cpu.state.prefetch[0] as i16 as i32 as u32,
        EA_DIPC => {
            let disp = cpu.state.prefetch[0] as i16 as i32;
            base_pc.wrapping_add(disp as u32)
        }
        EA_IXPC => {
            let ext = cpu.state.prefetch[0];
            let base = base_pc;
            let disp = (ext & 0xFF) as i8 as i32;
            let idx_reg = ((ext >> 12) & 7) as usize;
            let is_a = (ext & 0x8000) != 0;
            let is_long = (ext & 0x0800) != 0;
            let idx_val = if is_a { cpu.state.read_a(idx_reg) } else { cpu.state.d[idx_reg] };
            let idx_ext = if is_long { idx_val as i32 } else { (idx_val as i16) as i32 };
            base.wrapping_add(disp as u32).wrapping_add(idx_ext as u32)
        }
        _ => 0,
    }
}

/// Cycle-exact JMP handler parameterized by control addressing mode
pub fn op_jmp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let reg = (cpu.state.ir & 7) as usize;
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);

    if m == EA_AL {
        // JMP (xxx).L: 12 CPU clocks (4 extension read + 2 prefetch cycles)
        match cpu.state.micro.micro_step {
            0 => {
                let ext_pc = base_pc.wrapping_add(2);
                cpu.initiate_bus_cycle(BusCycle::new_read(ext_pc, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                let hi = (cpu.state.prefetch[0] as u32) << 16;
                let lo = cpu.state.micro.last_read as u32;
                let target = hi | lo;
                if (target & 1) != 0 {
                    return trigger_address_error(cpu, target, true, true, bus);
                }
                cpu.state.micro.scratch[0] = target;
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else if m == EA_AI {
        // JMP (An): 8 CPU clocks (2 prefetch cycles)
        let target = resolve_control_target(cpu, m, reg, base_pc);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        // Modes with 1 extension word: (d16,An), (xxx).w, (d16,PC) -> 10 clocks (2 internal + 2 prefetch)
        // (d8,An,Xn), (d8,PC,Xn) -> 14 clocks (6 internal + 2 prefetch)
        let target = resolve_control_target(cpu, m, reg, base_pc);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        let internal_clocks = if m == EA_IX || m == EA_IXPC { 6 } else { 2 };
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.internal_clocks = internal_clocks;
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
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

// ============================================================================
// 4. JSR
// ============================================================================

/// Cycle-exact JSR handler parameterized by control addressing mode
pub fn op_jsr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let reg = (cpu.state.ir & 7) as usize;
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);
    let fc_d = data_fc(cpu);

    let (return_pc, target, internal_clocks) = match m {
        EA_AI => (base_pc, resolve_control_target(cpu, m, reg, base_pc), 0),
        EA_AL => {
            // Evaluated in step 0 & 1
            (base_pc.wrapping_add(4), 0, 0)
        }
        EA_IX | EA_IXPC => (base_pc.wrapping_add(2), resolve_control_target(cpu, m, reg, base_pc), 6),
        _ => (base_pc.wrapping_add(2), resolve_control_target(cpu, m, reg, base_pc), 2),
    };

    if m == EA_AL {
        // JSR (xxx).L: 20 CPU clocks (4 extension read + 4 prefetch target + 4 SP write + 4 SP+2 write + 4 prefetch target+2)
        match cpu.state.micro.micro_step {
            0 => {
                let ext_pc = base_pc.wrapping_add(2);
                cpu.initiate_bus_cycle(BusCycle::new_read(ext_pc, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                let hi = (cpu.state.prefetch[0] as u32) << 16;
                let lo = cpu.state.micro.last_read as u32;
                let target = hi | lo;
                if (target & 1) != 0 {
                    return trigger_address_error(cpu, target, true, true, bus);
                }
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            4 => {
                let target = cpu.state.micro.scratch[0];
                let new_ir = cpu.state.micro.scratch_prefetch;
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else if m == EA_AI {
        // JSR (An): 16 CPU clocks
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
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
                let new_ir = cpu.state.micro.scratch_prefetch;
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        // JSR with extension word: 18 or 22 CPU clocks
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.state.micro.internal_clocks = internal_clocks;
                StepResult::StepCompleted
            }
            1 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            4 => {
                let target = cpu.state.micro.scratch[0];
                let new_ir = cpu.state.micro.scratch_prefetch;
                cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

// ============================================================================
// 5. RTS
// ============================================================================

/// Cycle-exact RTS handler (16 CPU clocks / 8 CCKs)
pub fn op_rts(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let fc_d = data_fc(cpu);
    let fc_p = prog_fc(cpu);

    match cpu.state.micro.micro_step {
        0 => {
            let sp = cpu.state.a7();
            if (sp & 1) != 0 {
                return trigger_address_error(cpu, sp, true, false, bus);
            }
            cpu.initiate_bus_cycle(BusCycle::new_read(sp, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        1 => {
            let hi = (cpu.state.micro.last_read as u32) << 16;
            cpu.state.micro.scratch[0] = hi;
            let sp = cpu.state.a7().wrapping_add(2);
            cpu.initiate_bus_cycle(BusCycle::new_read(sp, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        2 => {
            let lo = cpu.state.micro.last_read as u32;
            let target = cpu.state.micro.scratch[0] | lo;
            cpu.state.set_a7(cpu.state.a7().wrapping_add(4));
            if (target & 1) != 0 {
                return trigger_address_error(cpu, target, true, true, bus);
            }
            cpu.state.micro.scratch[0] = target;
            cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
            StepResult::StepCompleted
        }
        3 => {
            let new_ir = cpu.state.micro.last_read;
            let target = cpu.state.micro.scratch[0];
            cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
            cpu.state.micro.mark_target_refill_retire(target, new_ir);
            StepResult::StepCompleted
        }
        _ => unreachable!(),
    }
}

// ============================================================================
// 6. TRAP
// ============================================================================

/// Cycle-exact TRAP handler (34 CPU clocks / 17 CCKs)
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
            cpu.state.sr |= 0x2000;
            cpu.state.sr &= !0x8000;
            cpu.state.micro.scratch[0] = return_pc;
            cpu.state.micro.scratch[1] = old_sr as u32;
            cpu.state.micro.scratch[2] = vector_addr;
            StepResult::StepCompleted
        }
        1 => {
            // Write return PC low word to SP - 2
            let sp = cpu.state.ssp;
            let lo = (cpu.state.micro.scratch[0] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(sp.wrapping_sub(2), lo, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        2 => {
            // Write old SR to SP - 6
            let sp = cpu.state.ssp;
            let sr = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(sp.wrapping_sub(6), sr, BusAccessSize::Word, fc_d));
            StepResult::StepCompleted
        }
        3 => {
            // Write return PC high word to SP - 4 and update ssp
            let sp = cpu.state.ssp;
            let hi = ((cpu.state.micro.scratch[0] >> 16) & 0xFFFF) as u16;
            cpu.state.ssp = sp.wrapping_sub(6);
            cpu.initiate_bus_cycle(BusCycle::new_write(sp.wrapping_sub(4), hi, BusAccessSize::Word, fc_d));
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
            cpu.initiate_bus_cycle(BusCycle::new_read(vec_addr.wrapping_add(2), BusAccessSize::Word, fc_d));
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
            cpu.initiate_bus_cycle(BusCycle::new_read(target.wrapping_add(2), BusAccessSize::Word, fc_p));
            cpu.state.micro.mark_target_refill_retire(target, new_ir);
            StepResult::StepCompleted
        }
        _ => unreachable!(),
    }
}
