//! Linearized cycle-exact ADDX and SUBX instruction implementations (Group 0xD and Group 0x9)
//!
//! Handles:
//! - Register-to-register: `ADDX Dy, Dx` and `SUBX Dy, Dx` (4 clocks for Byte/Word, 8 clocks for Long)
//! - Memory-to-memory: `ADDX -(Ay), -(Ax)` and `SUBX -(Ay), -(Ax)` (18 clocks for Byte/Word, 30 clocks for Long)
//!
//! Quirk note: In ADDX and SUBX, the Z flag is cleared if the result is non-zero,
//! but remains unchanged if the result is zero (preserving chained multi-precision zero status).

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::arithmetic::{execute_addx, execute_subx};
use crate::instructions::linear_ea::{bus_size_from_const, data_fc, size_from_const, trigger_address_error, SIZE_BYTE, SIZE_LONG, SIZE_WORD};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// 1. Register-to-Register: ADDX / SUBX Dy, Dx
// ============================================================================

/// Cycle-exact register-to-register ADDX Dy, Dx handler
pub fn op_addx_reg(
    cpu: &mut Cpu,
    _bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let rx = ((ir >> 9) & 7) as usize;
    let ry = (ir & 7) as usize;
    let size = size_from_const(s);

    if s == SIZE_LONG {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let src = cpu.state.d[ry];
                let dst = cpu.state.d[rx];
                let res = execute_addx(&mut cpu.state, src, dst, Size::Long);
                cpu.write_d_reg(rx, res, Size::Long);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let src = cpu.state.d[ry];
        let dst = cpu.state.d[rx];
        let res = execute_addx(&mut cpu.state, src, dst, size);
        cpu.write_d_reg(rx, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

/// Cycle-exact register-to-register SUBX Dy, Dx handler
pub fn op_subx_reg(
    cpu: &mut Cpu,
    _bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let rx = ((ir >> 9) & 7) as usize;
    let ry = (ir & 7) as usize;
    let size = size_from_const(s);

    if s == SIZE_LONG {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let src = cpu.state.d[ry];
                let dst = cpu.state.d[rx];
                let res = execute_subx(&mut cpu.state, src, dst, Size::Long);
                cpu.write_d_reg(rx, res, Size::Long);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let src = cpu.state.d[ry];
        let dst = cpu.state.d[rx];
        let res = execute_subx(&mut cpu.state, src, dst, size);
        cpu.write_d_reg(rx, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// 2. Memory-to-Memory Predecrement: ADDX / SUBX -(Ay), -(Ax)
// ============================================================================

/// Cycle-exact memory predecrement ADDX -(Ay), -(Ax) handler
pub fn op_addx_mem(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let bus_size = bus_size_from_const(s);
    let size = size_from_const(s);
    let fc = data_fc(cpu);
    let ry = (ir & 7) as usize;
    let rx = ((ir >> 9) & 7) as usize;

    if s == SIZE_LONG {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let orig_y = cpu.state.read_a(ry);
                let first_y = orig_y.wrapping_sub(2);
                if (first_y & 1) != 0 {
                    cpu.state.write_a(ry, first_y);
                    return trigger_address_error(cpu, first_y, true, false, bus);
                }
                let addr_y = orig_y.wrapping_sub(4);
                cpu.state.write_a(ry, addr_y);
                cpu.state.micro.scratch[0] = addr_y;
                StepResult::StepCompleted
            }
            1 => {
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    addr_y.wrapping_add(2),
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            2 => {
                let lo_y = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[1] = lo_y;
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, BusAccessSize::Word, fc));
                StepResult::StepCompleted
            }
            3 => {
                let hi_y = (cpu.state.micro.last_read as u32) << 16;
                cpu.state.micro.scratch[1] |= hi_y;
                let orig_x = cpu.state.read_a(rx);
                let first_x = orig_x.wrapping_sub(2);
                if (first_x & 1) != 0 {
                    cpu.state.write_a(rx, first_x);
                    return trigger_address_error(cpu, first_x, true, false, bus);
                }
                let addr_x = orig_x.wrapping_sub(4);
                cpu.state.write_a(rx, addr_x);
                cpu.state.micro.scratch[0] = addr_x;
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    addr_x.wrapping_add(2),
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            4 => {
                let lo_x = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[2] = lo_x;
                let addr_x = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, BusAccessSize::Word, fc));
                StepResult::StepCompleted
            }
            5 => {
                let hi_x = (cpu.state.micro.last_read as u32) << 16;
                let val_x = cpu.state.micro.scratch[2] | hi_x;
                let val_y = cpu.state.micro.scratch[1];
                let res = execute_addx(&mut cpu.state, val_y, val_x, Size::Long);
                cpu.state.micro.scratch[1] = res;
                let addr_x = cpu.state.micro.scratch[0];
                let low_word = (res & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(
                    addr_x.wrapping_add(2),
                    low_word,
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            6 => {
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            7 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let addr_x = cpu.state.micro.scratch[0];
                let hi_word = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(
                    addr_x,
                    hi_word,
                    BusAccessSize::Word,
                    fc,
                ));
                cpu.state.micro.mark_scratch_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let dec_y = if ry == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                let addr_y = cpu.state.read_a(ry).wrapping_sub(dec_y);
                cpu.state.write_a(ry, addr_y);
                if s == SIZE_WORD && (addr_y & 1) != 0 {
                    return trigger_address_error(cpu, addr_y, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_y;
                StepResult::StepCompleted
            }
            1 => {
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, bus_size, fc));
                StepResult::StepCompleted
            }
            2 => {
                let val_y = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[1] = val_y;
                let dec_x = if rx == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                let addr_x = cpu.state.read_a(rx).wrapping_sub(dec_x);
                cpu.state.write_a(rx, addr_x);
                if s == SIZE_WORD && (addr_x & 1) != 0 {
                    return trigger_address_error(cpu, addr_x, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_x;
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, bus_size, fc));
                StepResult::StepCompleted
            }
            3 => {
                let val_x = cpu.state.micro.last_read as u32;
                let val_y = cpu.state.micro.scratch[1];
                let res = execute_addx(&mut cpu.state, val_y, val_x, size);
                cpu.state.micro.scratch[1] = res;
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            4 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let addr_x = cpu.state.micro.scratch[0];
                let write_val = if s == SIZE_BYTE {
                    (cpu.state.micro.scratch[1] & 0xFF) as u16
                } else {
                    cpu.state.micro.scratch[1] as u16
                };
                cpu.initiate_bus_cycle(BusCycle::new_write(addr_x, write_val, bus_size, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

/// Cycle-exact memory predecrement SUBX -(Ay), -(Ax) handler
pub fn op_subx_mem(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let bus_size = bus_size_from_const(s);
    let size = size_from_const(s);
    let fc = data_fc(cpu);
    let ry = (ir & 7) as usize;
    let rx = ((ir >> 9) & 7) as usize;

    if s == SIZE_LONG {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let orig_y = cpu.state.read_a(ry);
                let first_y = orig_y.wrapping_sub(2);
                if (first_y & 1) != 0 {
                    cpu.state.write_a(ry, first_y);
                    return trigger_address_error(cpu, first_y, true, false, bus);
                }
                let addr_y = orig_y.wrapping_sub(4);
                cpu.state.write_a(ry, addr_y);
                cpu.state.micro.scratch[0] = addr_y;
                StepResult::StepCompleted
            }
            1 => {
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    addr_y.wrapping_add(2),
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            2 => {
                let lo_y = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[1] = lo_y;
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, BusAccessSize::Word, fc));
                StepResult::StepCompleted
            }
            3 => {
                let hi_y = (cpu.state.micro.last_read as u32) << 16;
                cpu.state.micro.scratch[1] |= hi_y;
                let orig_x = cpu.state.read_a(rx);
                let first_x = orig_x.wrapping_sub(2);
                if (first_x & 1) != 0 {
                    cpu.state.write_a(rx, first_x);
                    return trigger_address_error(cpu, first_x, true, false, bus);
                }
                let addr_x = orig_x.wrapping_sub(4);
                cpu.state.write_a(rx, addr_x);
                cpu.state.micro.scratch[0] = addr_x;
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    addr_x.wrapping_add(2),
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            4 => {
                let lo_x = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[2] = lo_x;
                let addr_x = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, BusAccessSize::Word, fc));
                StepResult::StepCompleted
            }
            5 => {
                let hi_x = (cpu.state.micro.last_read as u32) << 16;
                let val_x = cpu.state.micro.scratch[2] | hi_x;
                let val_y = cpu.state.micro.scratch[1];
                let res = execute_subx(&mut cpu.state, val_y, val_x, Size::Long);
                cpu.state.micro.scratch[1] = res;
                let addr_x = cpu.state.micro.scratch[0];
                let low_word = (res & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(
                    addr_x.wrapping_add(2),
                    low_word,
                    BusAccessSize::Word,
                    fc,
                ));
                StepResult::StepCompleted
            }
            6 => {
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            7 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let addr_x = cpu.state.micro.scratch[0];
                let hi_word = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(
                    addr_x,
                    hi_word,
                    BusAccessSize::Word,
                    fc,
                ));
                cpu.state.micro.mark_scratch_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.internal_clocks = 2;
                let dec_y = if ry == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                let addr_y = cpu.state.read_a(ry).wrapping_sub(dec_y);
                cpu.state.write_a(ry, addr_y);
                if s == SIZE_WORD && (addr_y & 1) != 0 {
                    return trigger_address_error(cpu, addr_y, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_y;
                StepResult::StepCompleted
            }
            1 => {
                let addr_y = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_y, bus_size, fc));
                StepResult::StepCompleted
            }
            2 => {
                let val_y = cpu.state.micro.last_read as u32;
                cpu.state.micro.scratch[1] = val_y;
                let dec_x = if rx == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                let addr_x = cpu.state.read_a(rx).wrapping_sub(dec_x);
                cpu.state.write_a(rx, addr_x);
                if s == SIZE_WORD && (addr_x & 1) != 0 {
                    return trigger_address_error(cpu, addr_x, true, false, bus);
                }
                cpu.state.micro.scratch[0] = addr_x;
                cpu.initiate_bus_cycle(BusCycle::new_read(addr_x, bus_size, fc));
                StepResult::StepCompleted
            }
            3 => {
                let val_x = cpu.state.micro.last_read as u32;
                let val_y = cpu.state.micro.scratch[1];
                let res = execute_subx(&mut cpu.state, val_y, val_x, size);
                cpu.state.micro.scratch[1] = res;
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            4 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let addr_x = cpu.state.micro.scratch[0];
                let write_val = if s == SIZE_BYTE {
                    (cpu.state.micro.scratch[1] & 0xFF) as u16
                } else {
                    cpu.state.micro.scratch[1] as u16
                };
                cpu.initiate_bus_cycle(BusCycle::new_write(addr_x, write_val, bus_size, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}
