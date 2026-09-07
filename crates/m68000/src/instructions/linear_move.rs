//! Linear Data Movement (`MOVE` and `MOVEA`) execution handlers
//!
//! Provides compile-time constant parameterized handlers for all 12,288 opcodes
//! of the Motorola 68000 `MOVE.b/w/l` and `MOVEA.w/l` instruction family.
//! Decomposes execution into Color Clock (CCK1/CCK2) bus sub-cycles and micro-steps
//! with zero dynamic runtime branching.

use crate::core::{Cpu, StepResult};
use crate::instructions::linear_ea::{
    bus_size_from_const, data_fc, prefetch_extension, decode_ea_index, read_ea_operand, size_from_const,
    trigger_address_error, EA_AI, EA_AL, EA_AW, EA_DI, EA_AN, EA_DN, EA_IX, EA_PD, EA_PI, SIZE_BYTE,
    SIZE_LONG, SIZE_WORD,
};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// 1. Register Destination: MOVE <ea>, Dn and MOVEA <ea>, An
// ============================================================================

/// Compile-time parameterized handler for MOVE / MOVEA to register
///
/// - `S`: Operand size (`SIZE_BYTE`, `SIZE_WORD`, `SIZE_LONG`)
/// - `DST_M`: Destination register type (`EA_DN` for Dn, `EA_AN` for An / MOVEA)
/// - `SRC_M`: Source effective address mode (0..11)
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
    let dst_mode = ((ir >> 6) & 7) as u8;
    let dst_m = if dst_mode == 0 { EA_DN } else { EA_AN };
    let src_m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, src_m) {
        Ok(v) => v,
        Err(res) => return res,
    };

    let dst_reg = ((ir >> 9) & 7) as usize;

    if dst_m == EA_DN {
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
        cpu.state.set_n(n);
        cpu.state.set_z(z);
        cpu.state.set_v(false);
        cpu.state.set_c(false);
        // Extend flag (X) is completely unaffected

        cpu.write_d_reg(dst_reg, val, size_from_const(s));
    } else {
        // MOVEA <ea>, An: Sign-extend Word, Long written directly; CCR untouched
        let final_val = if s == SIZE_WORD {
            val as i16 as i32 as u32
        } else {
            val
        };
        cpu.state.write_a(dst_reg, final_val);
    }

    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}

// ============================================================================
// 2. Memory Destination: MOVE <ea_src>, <ea_dst>
// ============================================================================

/// Sub-cycle destination address resolution and write pipeline
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
                // Section 7.6: An is never incremented if write address is unaligned
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
                // Section 7.8: Long Predecrement writes low word to An - 2, high word to An - 4
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
                // Section 7.8: Byte/Word Predecrement prefetches next opcode before writing
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
                prefetch_extension(cpu);
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
                    cpu.state.d[x_reg]
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
                prefetch_extension(cpu);
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
                prefetch_extension(cpu);
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
                prefetch_extension(cpu);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.micro.scratch[0] |= cpu.state.micro.last_read as u32;
                prefetch_extension(cpu);
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

/// Compile-time parameterized handler for MOVE to memory destination
///
/// - `S`: Operand size (`SIZE_BYTE`, `SIZE_WORD`, `SIZE_LONG`)
/// - `dst_m`: Destination memory addressing mode (`EA_AI`..`EA_AL`)
/// - `SRC_M`: Source effective address mode (0..11)
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

        // Section 7.9: Condition codes reflect the full transferred value
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
        cpu.state.set_n(n);
        cpu.state.set_z(z);
        cpu.state.set_v(false);
        cpu.state.set_c(false);
        // Extend flag (X) is completely unaffected

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
