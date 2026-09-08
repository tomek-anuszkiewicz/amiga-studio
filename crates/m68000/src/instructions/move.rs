//! MOVE instruction handlers and CCR updates
//!
//! Handles `MOVE.B`, `MOVE.W`, and `MOVE.L` data movement instructions.
//! Updates Condition Codes (N, Z set according to result, V, C cleared, X unaffected).

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    bus_size_from_const, data_fc, decode_ea_index, read_ea_operand,
    size_from_const, trigger_address_error, EA_AI, EA_AL, EA_AW, EA_DI, EA_IX, EA_PD, EA_PI,
    SIZE_BYTE, SIZE_LONG, SIZE_WORD,
};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

/// Evaluates CCR updates for MOVE instruction (N and Z updated, V and C cleared, X unchanged)
#[inline]
pub fn update_ccr_move(state: &mut CpuState, val: u32, size: Size) {
    let (is_negative, is_zero) = match size {
        Size::Byte => ((val as u8 & 0x80) != 0, (val as u8) == 0),
        Size::Word => ((val as u16 & 0x8000) != 0, (val as u16) == 0),
        Size::Long => ((val & 0x8000_0000) != 0, val == 0),
    };
    state.set_ccr_nz_clear_vc(is_negative, is_zero);
}

/// Execution handler for MOVE <ea>, Dn
pub fn op_move_to_reg(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = match (ir >> 12) & 3 {
        1 => SIZE_BYTE,
        3 => SIZE_WORD,
        _ => SIZE_LONG,
    };
    let src_m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, src_m) {
        Ok(v) => v,
        Err(res) => return res,
    };

    let dst_reg = ((ir >> 9) & 7) as usize;

    // MOVE <ea>, Dn: Evaluate CCR condition codes
    let n = match s {
        SIZE_BYTE => ((val as u8) as i8) < 0,
        SIZE_WORD => ((val as u16) as i16) < 0,
        _ => (val as i32) < 0,
    };
    let z = match s {
        SIZE_BYTE => (val as u8) == 0,
        SIZE_WORD => (val as u16) == 0,
        _ => val == 0,
    };
    cpu.state.set_ccr_nz_clear_vc(n, z);

    cpu.write_d_reg(dst_reg, val, size_from_const(s));

    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}

/// Sub-cycle destination address resolution and write pipeline for memory destination
#[inline(always)]
fn write_move_dst(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    s: u8,
    dst_m: u8,
    val: u32,
    dst_step: u16,
) -> StepResult {
    let dst_reg = ((cpu.state.ir >> 9) & 7) as usize;
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    match dst_m {
        EA_AI => match dst_step {
            0 => {
                let addr = cpu.state.read_a(dst_reg);
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                if s == SIZE_LONG {
                    cpu.state.micro.scratch[0] = addr;
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            1 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            2 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        EA_PI => match dst_step {
            0 => {
                let addr = cpu.state.read_a(dst_reg);
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                let inc = if s == SIZE_LONG {
                    4
                } else if dst_reg == 7 && s == SIZE_BYTE {
                    2
                } else if s == SIZE_WORD {
                    2
                } else {
                    1
                };
                cpu.state.write_a(dst_reg, addr.wrapping_add(inc));
                if s == SIZE_LONG {
                    cpu.state.micro.scratch[0] = addr;
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            1 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            2 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        EA_PD => {
            if s == SIZE_LONG {
                match dst_step {
                    0 => {
                        let addr2 = cpu.state.read_a(dst_reg).wrapping_sub(2);
                        cpu.state.write_a(dst_reg, addr2);
                        if (addr2 & 1) != 0 {
                            return trigger_address_error(cpu, addr2, false, false, bus);
                        }
                        let addr4 = addr2.wrapping_sub(2);
                        cpu.state.micro.scratch[0] = addr4;
                        cpu.initiate_bus_cycle(BusCycle::new_write(
                            addr2,
                            (val & 0xFFFF) as u16,
                            BusAccessSize::Word,
                            fc,
                        ));
                        StepResult::StepCompleted
                    }
                    1 => {
                        let addr4 = cpu.state.micro.scratch[0];
                        cpu.state.write_a(dst_reg, addr4);
                        cpu.initiate_bus_cycle(BusCycle::new_write(
                            addr4,
                            (val >> 16) as u16,
                            BusAccessSize::Word,
                            fc,
                        ));
                        StepResult::StepCompleted
                    }
                    2 => {
                        cpu.initiate_prefetch();
                        cpu.state.micro.mark_standard_prefetch_retire();
                        StepResult::StepCompleted
                    }
                    _ => unreachable!(),
                }
            } else {
                match dst_step {
                    0 => {
                        let dec = if dst_reg == 7 && s == SIZE_BYTE {
                            2
                        } else if s == SIZE_WORD {
                            2
                        } else {
                            1
                        };
                        let addr = cpu.state.read_a(dst_reg).wrapping_sub(dec);
                        cpu.state.write_a(dst_reg, addr);
                        cpu.state.micro.scratch[0] = addr;
                        cpu.initiate_prefetch();
                        StepResult::StepCompleted
                    }
                    1 => {
                        let prefetched = cpu.state.micro.last_read;
                        cpu.state.micro.scratch_prefetch = prefetched;
                        let addr = cpu.state.micro.scratch[0];
                        if s != SIZE_BYTE && (addr & 1) != 0 {
                            cpu.state.ir = cpu.state.prefetch[0];
                            return trigger_address_error(cpu, addr, false, false, bus);
                        }
                        let write_val = if s == SIZE_BYTE {
                            (val & 0xFF) as u16
                        } else {
                            val as u16
                        };
                        cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                        cpu.state.micro.mark_scratch_prefetch_retire();
                        StepResult::StepCompleted
                    }
                    _ => unreachable!(),
                }
            }
        }

        EA_DI => match dst_step {
            0 => {
                let disp = cpu.state.prefetch[0] as i16 as i32;
                let addr = cpu.state.read_a(dst_reg).wrapping_add(disp as u32);
                cpu.state.micro.scratch[0] = addr;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                let addr = cpu.state.micro.scratch[0];
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                if s == SIZE_LONG {
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            2 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            3 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        EA_IX => match dst_step {
            0 => {
                let ext = cpu.state.prefetch[0];
                let disp = (ext as i8) as i32;
                let x_reg = ((ext >> 12) & 7) as usize;
                let is_a = (ext & 0x8000) != 0;
                let is_long = (ext & 0x0800) != 0;
                let x_val = if is_a {
                    cpu.state.read_a(x_reg)
                } else {
                    cpu.state.d_long(x_reg)
                };
                let x_idx = if is_long {
                    x_val as i32
                } else {
                    (x_val as i16) as i32
                };
                let addr = cpu
                    .state
                    .read_a(dst_reg)
                    .wrapping_add(disp.wrapping_add(x_idx) as u32);
                cpu.state.micro.scratch[0] = addr;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                cpu.record_internal_clocks(2);
                StepResult::StepCompleted
            }
            2 => {
                let addr = cpu.state.micro.scratch[0];
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                if s == SIZE_LONG {
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            3 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            4 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        EA_AW => match dst_step {
            0 => {
                let addr = (cpu.state.prefetch[0] as i16 as i32) as u32;
                cpu.state.micro.scratch[0] = addr;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                let addr = cpu.state.micro.scratch[0];
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                if s == SIZE_LONG {
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            2 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            3 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        EA_AL => match dst_step {
            0 => {
                cpu.state.micro.scratch[0] = (cpu.state.prefetch[0] as u32) << 16;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.micro.scratch[0] |= cpu.state.micro.last_read as u32;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                let addr = cpu.state.micro.scratch[0];
                if s != SIZE_BYTE && (addr & 1) != 0 {
                    return trigger_address_error(cpu, addr, false, false, bus);
                }
                if s == SIZE_LONG {
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr,
                        (val >> 16) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                } else {
                    let write_val = if s == SIZE_BYTE {
                        (val & 0xFF) as u16
                    } else {
                        val as u16
                    };
                    cpu.initiate_bus_cycle(BusCycle::new_write(addr, write_val, bus_size, fc));
                }
                StepResult::StepCompleted
            }
            3 => {
                if s == SIZE_LONG {
                    let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_write(
                        addr2,
                        (val & 0xFFFF) as u16,
                        BusAccessSize::Word,
                        fc,
                    ));
                    StepResult::StepCompleted
                } else {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
            }
            4 if s == SIZE_LONG => {
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        },

        _ => unreachable!(),
    }
}

/// Execution handler for MOVE to memory destination
pub fn op_move_to_mem(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = match (ir >> 12) & 3 {
        1 => SIZE_BYTE,
        3 => SIZE_WORD,
        _ => SIZE_LONG,
    };
    let dst_mode = ((ir >> 6) & 7) as u8;
    let dst_reg = ((ir >> 9) & 7) as u8;
    let dst_m = decode_ea_index(dst_mode, dst_reg);
    let src_m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    let phase = cpu.state.micro.scratch[2];
    if phase == 0 {
        let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, src_m) {
            Ok(v) => v,
            Err(res) => return res,
        };

        let n = match s {
            SIZE_BYTE => ((val as u8) as i8) < 0,
            SIZE_WORD => ((val as u16) as i16) < 0,
            _ => (val as i32) < 0,
        };
        let z = match s {
            SIZE_BYTE => (val as u8) == 0,
            SIZE_WORD => (val as u16) == 0,
            _ => val == 0,
        };
        cpu.state.set_ccr_nz_clear_vc(n, z);

        cpu.state.micro.scratch[1] = val;
        cpu.state.micro.scratch[2] = 1; // Enter destination phase
        cpu.state.micro.scratch[3] = 0; // Destination step 0

        write_move_dst(cpu, bus, s, dst_m, val, 0)
    } else {
        let val = cpu.state.micro.scratch[1];
        let dst_step = (cpu.state.micro.scratch[3] as u16).wrapping_add(1);
        cpu.state.micro.scratch[3] = dst_step as u32;
        write_move_dst(cpu, bus, s, dst_m, val, dst_step)
    }
}
