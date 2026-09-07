//! Linear Effective Address (EA) sub-cycle execution primitives
//!
//! Provides compile-time constant parameterized effective address decoding,
//! address resolution, and bus cycle sequencing without cascaded runtime branching.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use memory_bus::{function_code, BusAccessSize, BusCycle, MemoryBus};

// --- Compile-Time EA Mode Constants ---
pub const EA_DN: u8 = 0;
pub const EA_AN: u8 = 1;
pub const EA_AI: u8 = 2;
pub const EA_PI: u8 = 3;
pub const EA_PD: u8 = 4;
pub const EA_DI: u8 = 5;
pub const EA_IX: u8 = 6;
pub const EA_AW: u8 = 7;
pub const EA_AL: u8 = 8;
pub const EA_DIPC: u8 = 9;
pub const EA_IXPC: u8 = 10;
pub const EA_IMM: u8 = 11;

// --- Compile-Time Size Constants ---
pub const SIZE_BYTE: u8 = 0;
pub const SIZE_WORD: u8 = 1;
pub const SIZE_LONG: u8 = 2;

/// Maps standard M68000 (mode: 3 bits, reg: 3 bits) to linear EA constant
#[inline(always)]
pub const fn decode_ea_index(mode: u8, reg: u8) -> u8 {
    match mode {
        0 => EA_DN,
        1 => EA_AN,
        2 => EA_AI,
        3 => EA_PI,
        4 => EA_PD,
        5 => EA_DI,
        6 => EA_IX,
        7 => match reg {
            0 => EA_AW,
            1 => EA_AL,
            2 => EA_DIPC,
            3 => EA_IXPC,
            4 => EA_IMM,
            _ => 255,
        },
        _ => 255,
    }
}

/// Returns true if the linear EA mode accesses memory (rather than Dn or An registers)
#[inline(always)]
pub const fn is_memory_ea(mode: u8) -> bool {
    mode >= EA_AI && mode <= EA_IXPC
}

/// Returns data space Function Code (FC 1 for user, FC 5 for supervisor)
#[inline(always)]
pub fn data_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        function_code::SUPERVISOR_DATA
    } else {
        function_code::USER_DATA
    }
}

/// Returns program space Function Code (FC 2 for user, FC 6 for supervisor)
#[inline(always)]
pub fn prog_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        function_code::SUPERVISOR_PROGRAM
    } else {
        function_code::USER_PROGRAM
    }
}

/// Cold exception path for Group 0 Address Error on unaligned bus transfers
#[inline(never)]
pub fn trigger_address_error(
    cpu: &mut Cpu,
    addr: u32,
    is_read: bool,
    is_program_space: bool,
    bus: &mut MemoryBus,
) -> StepResult {
    let fc = if is_program_space {
        prog_fc(cpu)
    } else {
        data_fc(cpu)
    };
    // 8 clock periods spent up to and including the bus fault attempt
    cpu.instruction_clocks = cpu.instruction_clocks.wrapping_add(8);
    cpu.total_clocks = cpu.total_clocks.wrapping_add(8);
    cpu.handle_address_error_fc(addr, is_read, fc, bus);
    StepResult::InstructionCompleted
}

/// Prefetches the next extension word from PC and advances PC by 2
#[inline(always)]
pub fn prefetch_extension(cpu: &mut Cpu) {
    cpu.initiate_prefetch();
    cpu.state.pc = cpu.state.pc.wrapping_add(2);
}

/// Converts compile-time size constant to Size enum
#[inline(always)]
pub const fn size_from_const(s: u8) -> Size {
    match s {
        SIZE_BYTE => Size::Byte,
        SIZE_WORD => Size::Word,
        _ => Size::Long,
    }
}

/// Converts compile-time size constant to BusAccessSize enum
#[inline(always)]
pub const fn bus_size_from_const(s: u8) -> BusAccessSize {
    match s {
        SIZE_BYTE => BusAccessSize::Byte,
        _ => BusAccessSize::Word,
    }
}

/// Sub-cycle linear EA operand reader parameterized by compile-time Size (S) and Mode (M)
pub fn read_ea_operand(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    step: u16,
    s: u8,
    m: u8,
) -> Result<u32, StepResult> {
    match m {
        EA_DN => {
            let reg = (cpu.state.ir & 7) as usize;
            let val = match s {
                SIZE_BYTE => cpu.state.d[reg] & 0xFF,
                SIZE_WORD => cpu.state.d[reg] & 0xFFFF,
                _ => cpu.state.d[reg],
            };
            Ok(val)
        }
        EA_AN => {
            let reg = (cpu.state.ir & 7) as usize;
            let val = match s {
                SIZE_BYTE => cpu.state.read_a(reg) & 0xFF,
                SIZE_WORD => cpu.state.read_a(reg) & 0xFFFF,
                _ => cpu.state.read_a(reg),
            };
            Ok(val)
        }
        EA_IMM => {
            if s == SIZE_LONG {
                match step {
                    0 => {
                        let hi = cpu.state.prefetch[0];
                        cpu.state.micro.scratch[1] = (hi as u32) << 16;
                        prefetch_extension(cpu);
                        Err(StepResult::StepCompleted)
                    }
                    1 => {
                        let lo = cpu.state.micro.last_read;
                        let val = cpu.state.micro.scratch[1] | (lo as u32);
                        cpu.state.micro.scratch[1] = val;
                        cpu.state.prefetch[0] = lo;
                        cpu.state.pc = cpu.state.pc.wrapping_add(2);
                        Ok(val)
                    }
                    _ => unreachable!(),
                }
            } else {
                let val = match s {
                    SIZE_BYTE => (cpu.state.prefetch[0] & 0xFF) as u32,
                    _ => cpu.state.prefetch[0] as u32,
                };
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                Ok(val)
            }
        }
        EA_AI => {
            let reg = (cpu.state.ir & 7) as usize;
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    let addr = cpu.state.read_a(reg);
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.state.micro.scratch[0] = addr;
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                2 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_PI => {
            let reg = (cpu.state.ir & 7) as usize;
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    let addr = cpu.state.read_a(reg);
                    let inc = if s == SIZE_LONG {
                        4
                    } else if reg == 7 && s == SIZE_BYTE {
                        2
                    } else if s == SIZE_WORD {
                        2
                    } else {
                        1
                    };
                    cpu.state.write_a(reg, addr.wrapping_add(inc));
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.state.micro.scratch[0] = addr;
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                2 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_PD => {
            let reg = (cpu.state.ir & 7) as usize;
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    let dec = if s == SIZE_LONG {
                        4
                    } else if reg == 7 && s == SIZE_BYTE {
                        2
                    } else if s == SIZE_WORD {
                        2
                    } else {
                        1
                    };
                    let addr = cpu.state.read_a(reg).wrapping_sub(dec);
                    cpu.state.write_a(reg, addr);
                    cpu.state.micro.scratch[0] = addr;
                    cpu.record_internal_clocks(2);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                3 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_DI => {
            let reg = (cpu.state.ir & 7) as usize;
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    let disp = cpu.state.prefetch[0] as i16 as i32;
                    cpu.state.micro.scratch[0] = cpu.state.read_a(reg).wrapping_add(disp as u32);
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                3 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_IX => {
            let reg = (cpu.state.ir & 7) as usize;
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
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
                    cpu.state.micro.scratch[0] = cpu
                        .state
                        .read_a(reg)
                        .wrapping_add(disp.wrapping_add(x_idx) as u32);
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    cpu.record_internal_clocks(2);
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                3 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                4 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_AW => {
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    cpu.state.micro.scratch[0] = (cpu.state.prefetch[0] as i16 as i32) as u32;
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                3 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_AL => {
            let bus_size = bus_size_from_const(s);
            let fc = data_fc(cpu);
            match step {
                0 => {
                    cpu.state.micro.scratch[0] = (cpu.state.prefetch[0] as u32) << 16;
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.micro.scratch[0] |= cpu.state.micro.last_read as u32;
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, false, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                3 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                4 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_DIPC => {
            let bus_size = bus_size_from_const(s);
            let fc = prog_fc(cpu);
            let pc_base = cpu.state.pc.wrapping_sub(2);
            match step {
                0 => {
                    let disp = cpu.state.prefetch[0] as i16 as i32;
                    cpu.state.micro.scratch[0] = pc_base.wrapping_add(disp as u32);
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, true, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                3 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        EA_IXPC => {
            let bus_size = bus_size_from_const(s);
            let fc = prog_fc(cpu);
            let pc_base = cpu.state.pc.wrapping_sub(2);
            match step {
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
                    cpu.state.micro.scratch[0] =
                        pc_base.wrapping_add(disp.wrapping_add(x_idx) as u32);
                    prefetch_extension(cpu);
                    Err(StepResult::StepCompleted)
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    cpu.record_internal_clocks(2);
                    Err(StepResult::StepCompleted)
                }
                2 => {
                    let addr = cpu.state.micro.scratch[0];
                    if (s != SIZE_BYTE || s == SIZE_LONG) && (addr & 1) != 0 {
                        return Err(trigger_address_error(cpu, addr, true, true, bus));
                    }
                    cpu.initiate_bus_cycle(BusCycle::new_read(addr, bus_size, fc));
                    Err(StepResult::StepCompleted)
                }
                3 => {
                    if s == SIZE_LONG {
                        cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                        let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
                        cpu.initiate_bus_cycle(BusCycle::new_read(addr2, bus_size, fc));
                        Err(StepResult::StepCompleted)
                    } else {
                        let val = match s {
                            SIZE_BYTE => (cpu.state.micro.last_read & 0xFF) as u32,
                            _ => cpu.state.micro.last_read as u32,
                        };
                        cpu.state.micro.scratch[1] = val;
                        Ok(val)
                    }
                }
                4 if s == SIZE_LONG => {
                    let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                    cpu.state.micro.scratch[1] = val;
                    Ok(val)
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}
