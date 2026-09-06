//! M68000 Effective Address (EA) decoding, calculation, and operand resolution

use crate::state::CpuState;

/// Operand size for M68000 instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Byte,
    Word,
    Long,
}

impl Size {
    #[inline]
    pub fn byte_count(self) -> u32 {
        match self {
            Size::Byte => 1,
            Size::Word => 2,
            Size::Long => 4,
        }
    }
}

/// Index register type for indexed addressing modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    Data,
    Address,
}

/// Index register specification from brief extension word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexReg {
    pub reg_type: IndexType,
    pub reg_idx: u8,
    pub is_long: bool,
}

impl IndexReg {
    /// Reads and sign-extends the index register value from CpuState
    pub fn read_val(self, state: &CpuState) -> u32 {
        let raw = match self.reg_type {
            IndexType::Data => state.d[self.reg_idx as usize],
            IndexType::Address => state.read_a(self.reg_idx as usize),
        };
        if self.is_long {
            raw
        } else {
            (raw as i16 as i32) as u32
        }
    }
}

/// The 12 M68000 Effective Addressing modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    /// Mode 000: Data Register Direct (Dn)
    DataDirect(u8),
    /// Mode 001: Address Register Direct (An)
    AddressDirect(u8),
    /// Mode 010: Address Register Indirect ((An))
    AddressIndirect(u8),
    /// Mode 011: Address Register Indirect with Postincrement ((An)+)
    Postincrement(u8),
    /// Mode 100: Address Register Indirect with Predecrement (-(An))
    Predecrement(u8),
    /// Mode 101: Address Register Indirect with Displacement ((d16, An))
    Displacement(u8, i16),
    /// Mode 110: Address Register Indirect with Index ((d8, An, Xn))
    Indexed(u8, IndexReg, i8),
    /// Mode 111, Reg 000: Absolute Short ((xxx).W)
    AbsoluteShort(u32),
    /// Mode 111, Reg 001: Absolute Long ((xxx).L)
    AbsoluteLong(u32),
    /// Mode 111, Reg 010: Program Counter with Displacement ((d16, PC))
    PcDisplacement(u32, i16),
    /// Mode 111, Reg 011: Program Counter with Index ((d8, PC, Xn))
    PcIndexed(u32, IndexReg, i8),
    /// Mode 111, Reg 100: Immediate Data (#<data>)
    Immediate(u32),
}

/// Decodes brief extension word for indexed modes
pub fn decode_brief_extension(ext: u16) -> (IndexReg, i8) {
    let reg_type = if (ext & 0x8000) != 0 {
        IndexType::Address
    } else {
        IndexType::Data
    };
    let reg_idx = ((ext >> 12) & 0x07) as u8;
    let is_long = (ext & 0x0800) != 0;
    let displacement = (ext & 0x00FF) as i8;

    (
        IndexReg {
            reg_type,
            reg_idx,
            is_long,
        },
        displacement,
    )
}

/// Effective Address resolution error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EaError {
    AddressError { addr: u32, is_read: bool },
    IllegalAddressingMode,
}

impl AddressingMode {
    /// Decodes an effective address mode from 6-bit EA field (mode = bits 5-3, reg = bits 2-0)
    pub fn decode(
        mode: u8,
        reg: u8,
        size: Size,
        pc: u32,
        mut read_ext: impl FnMut() -> u16,
    ) -> Result<Self, EaError> {
        match mode & 0x07 {
            0 => Ok(AddressingMode::DataDirect(reg & 0x07)),
            1 => Ok(AddressingMode::AddressDirect(reg & 0x07)),
            2 => Ok(AddressingMode::AddressIndirect(reg & 0x07)),
            3 => Ok(AddressingMode::Postincrement(reg & 0x07)),
            4 => Ok(AddressingMode::Predecrement(reg & 0x07)),
            5 => {
                let d16 = read_ext() as i16;
                Ok(AddressingMode::Displacement(reg & 0x07, d16))
            }
            6 => {
                let ext = read_ext();
                let (index, d8) = decode_brief_extension(ext);
                Ok(AddressingMode::Indexed(reg & 0x07, index, d8))
            }
            7 => match reg & 0x07 {
                0 => {
                    let word = read_ext();
                    // Sign-extend 16-bit to 32-bit
                    let addr = (word as i16 as i32 as u32) & 0x00FF_FFFF;
                    Ok(AddressingMode::AbsoluteShort(addr))
                }
                1 => {
                    let hi = read_ext();
                    let lo = read_ext();
                    let addr = (((hi as u32) << 16) | (lo as u32)) & 0x00FF_FFFF;
                    Ok(AddressingMode::AbsoluteLong(addr))
                }
                2 => {
                    // Extension word address is current PC
                    let ext_pc = pc;
                    let d16 = read_ext() as i16;
                    Ok(AddressingMode::PcDisplacement(ext_pc, d16))
                }
                3 => {
                    let ext_pc = pc;
                    let ext = read_ext();
                    let (index, d8) = decode_brief_extension(ext);
                    Ok(AddressingMode::PcIndexed(ext_pc, index, d8))
                }
                4 => {
                    let val = match size {
                        Size::Byte => (read_ext() & 0xFF) as u32,
                        Size::Word => read_ext() as u32,
                        Size::Long => {
                            let hi = read_ext();
                            let lo = read_ext();
                            ((hi as u32) << 16) | (lo as u32)
                        }
                    };
                    Ok(AddressingMode::Immediate(val))
                }
                _ => Err(EaError::IllegalAddressingMode),
            },
            _ => Err(EaError::IllegalAddressingMode),
        }
    }

    /// Resolves the target memory address for memory-based addressing modes
    pub fn resolve_address(&self, state: &mut CpuState, size: Size) -> Result<u32, EaError> {
        let addr = match *self {
            AddressingMode::AddressIndirect(reg) => state.read_a(reg as usize),
            AddressingMode::Postincrement(reg) => {
                let a = state.read_a(reg as usize);
                // Stack Pointer A7 Quirk: On byte ops, A7 adjusts by 2 to keep stack word-aligned
                let delta = if size == Size::Byte && reg == 7 {
                    2
                } else {
                    size.byte_count()
                };
                state.write_a(reg as usize, a.wrapping_add(delta));
                a
            }
            AddressingMode::Predecrement(reg) => {
                let a = state.read_a(reg as usize);
                let delta = if size == Size::Byte && reg == 7 {
                    2
                } else {
                    size.byte_count()
                };
                let new_a = a.wrapping_sub(delta);
                state.write_a(reg as usize, new_a);
                new_a
            }
            AddressingMode::Displacement(reg, d16) => {
                let a = state.read_a(reg as usize);
                a.wrapping_add(d16 as i32 as u32)
            }
            AddressingMode::Indexed(reg, index, d8) => {
                let a = state.read_a(reg as usize);
                let x = index.read_val(state);
                a.wrapping_add(x).wrapping_add(d8 as i32 as u32)
            }
            AddressingMode::AbsoluteShort(addr) => addr,
            AddressingMode::AbsoluteLong(addr) => addr,
            AddressingMode::PcDisplacement(pc, d16) => pc.wrapping_add(d16 as i32 as u32),
            AddressingMode::PcIndexed(pc, index, d8) => {
                let x = index.read_val(state);
                pc.wrapping_add(x).wrapping_add(d8 as i32 as u32)
            }
            AddressingMode::DataDirect(_)
            | AddressingMode::AddressDirect(_)
            | AddressingMode::Immediate(_) => {
                return Err(EaError::IllegalAddressingMode);
            }
        };

        let physical_addr = addr & 0x00FF_FFFF;

        // Check for unaligned word/long address error
        if size != Size::Byte && (physical_addr & 1) != 0 {
            return Err(EaError::AddressError {
                addr: physical_addr,
                is_read: true,
            });
        }

        Ok(physical_addr)
    }
}
