//! Motorola 68000 Micro-Step State Machine Type Definitions
//!
//! Defines the atomic `MicroStep` descriptor, `MicroAction` primitives,
//! ALU function pointers (`AluFn`), size enums, and opcode descriptors.

use crate::state::CpuState;
use memory_bus::BusAccessSize;
use serde::{Deserialize, Serialize};

/// Bitwise flags categorizing micro-step operations
pub mod flags {
    pub const NONE: u8 = 0;
    pub const READ: u8 = 1 << 0;
    pub const WRITE: u8 = 1 << 1;
    pub const PREFETCH: u8 = 1 << 2;
    pub const PROGRAM_SPACE: u8 = 1 << 3;
    pub const DATA_SPACE: u8 = 1 << 4;
}

/// Pure internal ALU operation.
/// Operates strictly on `CpuState` using pre-decoded register indices.
pub type AluFn = fn(state: &mut CpuState, reg_src: u8, reg_dst: u8);

/// Returns data space Function Code (FC 1 for user, FC 5 for supervisor)
#[inline(always)]
pub fn data_fc(state: &CpuState) -> u8 {
    if state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_DATA
    } else {
        memory_bus::function_code::USER_DATA
    }
}

/// Returns program space Function Code (FC 2 for user, FC 6 for supervisor)
#[inline(always)]
pub fn prog_fc(state: &CpuState) -> u8 {
    if state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_PROGRAM
    } else {
        memory_bus::function_code::USER_PROGRAM
    }
}

/// Operand transfer size for M68000 instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Byte,
    Word,
    Long,
}

impl Size {
    #[inline(always)]
    pub fn byte_count(self) -> u32 {
        match self {
            Size::Byte => 1,
            Size::Word => 2,
            Size::Long => 4,
        }
    }
}

/// Specialized atomic bus action or internal CPU stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MicroAction {
    /// Internal instantaneous ALU operation or EA calculation (0 CCKs)
    Alu,
    /// Evaluate Bcc condition; sets internal_clocks and routes to taken vs not-taken steps
    BranchEval,

    // --- Operand Memory Reads (Data Space) ---
    /// Read 8-bit Byte from ea_addr into last_read (strobe based on ea_addr & 1)
    BusReadByte,
    /// Read 16-bit Word from ea_addr into last_read (both strobes asserted)
    BusReadWord,
    /// Read 16-bit High Word of 32-bit operand from ea_addr into scratch[0]
    BusReadLongHigh,
    /// Read 16-bit Low Word of 32-bit operand from ea_addr + 2, assemble into last_read
    BusReadLongLow,

    // --- Operand Memory Writes (Data Space) ---
    /// Write 8-bit Byte (write_buffer & 0xFF) to ea_addr (preserves unaddressed byte)
    BusWriteByte,
    /// Write 16-bit Word (write_buffer & 0xFFFF) to ea_addr (both strobes asserted)
    BusWriteWord,
    /// Write 16-bit High Word ((write_buffer >> 16) & 0xFFFF) to ea_addr
    BusWriteLongHigh,
    /// Write 16-bit Low Word (write_buffer & 0xFFFF) to ea_addr + 2
    BusWriteLongLow,

    // --- Stack Operations (Data Space) ---
    /// Pop 16-bit word from stack (SP) and increment SP += 2
    BusPopStack,
    /// Pop 16-bit high word of 32-bit address from stack (SP) and increment SP += 2 into scratch[0]
    BusPopStackHigh,
    /// Pop 16-bit low word of 32-bit address from stack (SP), assemble target into ea_addr, and increment SP += 2
    BusPopStackLow,
    /// Push 16-bit Most Significant Word to -(SP)
    BusPushStackHigh,
    /// Push 16-bit Least Significant Word to -(SP)
    BusPushStackLow,
    /// Push 16-bit Least Significant Word to -(SP) and retire with scratch_prefetch (e.g. PEA)
    BusPushStackLowAndRetire,

    // --- Instruction Prefetch & Pipeline Refill (Program Space) ---
    /// Fetch extension word from PC and advance PC += 2
    FetchExtension,
    /// Class 0 RMW: Fetch next opcode into scratch_prefetch before writing memory result
    BusPrefetchToScratch,
    /// Pipeline Refill Cycle 1: Fetch target opcode from ea_addr into scratch_prefetch
    BusReadTargetOpcode,
    /// Pipeline Refill Cycle 2: Fetch target prefetch from ea_addr + 2, set PC = ea_addr + 4, retire
    PrefetchTargetAndRetire,
    /// Standard sequential prefetch: ir = prefetch[0], prefetch[0] = last_read, PC += 2, retire
    PrefetchNextOpcodeAndRetire,
    /// Write 16-bit Word to ea_addr and retire at CCK2 (used by Class 0 RMW)
    BusWriteWordAndRetire,
    /// Write 8-bit Byte to ea_addr and retire at CCK2 (used by Class 0 RMW)
    BusWriteByteAndRetire,
    /// Write 16-bit Low Word to ea_addr + 2 and retire at CCK2 (used by Class 0 Long RMW)
    BusWriteLongLowAndRetire,
    /// Write 16-bit High Word to ea_addr and retire at CCK2 (used by MOVE.l -(An))
    BusWriteLongHighAndRetire,

    // --- Multi-register block transfer ---
    /// Multi-register block transfer step (loops until register mask in scratch[0] is zero)
    MovemTransfer,

    // --- System Register & Exception Operations ---
    OriToCcr,
    OriToSr,
    AndiToCcr,
    AndiToSr,
    EoriToCcr,
    EoriToSr,
    Trap,
}

/// Stateless, cache-dense atomic micro-step descriptor (4 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MicroStep {
    /// Atomic bus action or instantaneous step type
    pub action: MicroAction,
    /// Function pointer for ALU operations (None for pure bus steps)
    pub alu_fn: Option<AluFn>,
    /// Base CPU clocks consumed (4 for bus cycles, 0 for instantaneous ALU)
    pub base_clocks: u8,
    /// Step flags (Read, Write, Prefetch, ProgramSpace, DataSpace)
    pub flags: u8,
}

impl PartialEq for MicroStep {
    fn eq(&self, other: &Self) -> bool {
        self.action == other.action
            && (match (self.alu_fn, other.alu_fn) {
                (None, None) => true,
                (Some(a), Some(b)) => a as usize == b as usize,
                _ => false,
            })
            && self.base_clocks == other.base_clocks
            && self.flags == other.flags
    }
}

impl Eq for MicroStep {}

impl MicroStep {
    /// Creates an instantaneous ALU micro-step (0 base clocks)
    #[inline(always)]
    pub const fn alu(alu_fn: AluFn) -> Self {
        Self {
            action: MicroAction::Alu,
            alu_fn: Some(alu_fn),
            base_clocks: 0,
            flags: 0,
        }
    }

    /// Creates a generic bus cycle micro-step
    #[inline(always)]
    pub const fn bus(action: MicroAction, base_clocks: u8, flags: u8) -> Self {
        Self {
            action,
            alu_fn: None,
            base_clocks,
            flags,
        }
    }

    /// Standard 16-bit Word read from `ea_addr`
    #[inline(always)]
    pub const fn bus_read_word() -> Self {
        Self {
            action: MicroAction::BusReadWord,
            alu_fn: None,
            base_clocks: 4,
            flags: flags::READ | flags::DATA_SPACE,
        }
    }

    /// Standard 8-bit Byte read from `ea_addr`
    #[inline(always)]
    pub const fn bus_read_byte() -> Self {
        Self {
            action: MicroAction::BusReadByte,
            alu_fn: None,
            base_clocks: 4,
            flags: flags::READ | flags::DATA_SPACE,
        }
    }

    /// Read high word of 32-bit long operand from `ea_addr`
    #[inline(always)]
    pub const fn bus_read_long_high() -> Self {
        Self {
            action: MicroAction::BusReadLongHigh,
            alu_fn: None,
            base_clocks: 4,
            flags: flags::READ | flags::DATA_SPACE,
        }
    }

    /// Read low word of 32-bit long operand from `ea_addr + 2`
    #[inline(always)]
    pub const fn bus_read_long_low() -> Self {
        Self {
            action: MicroAction::BusReadLongLow,
            alu_fn: None,
            base_clocks: 4,
            flags: flags::READ | flags::DATA_SPACE,
        }
    }
}

/// Static descriptor mapping an opcode to its slice of MicroSteps and registers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcodeDescriptor {
    pub steps: &'static [MicroStep],
    pub reg_src: u8,
    pub reg_dst: u8,
}

impl OpcodeDescriptor {
    pub const fn new(steps: &'static [MicroStep], reg_src: u8, reg_dst: u8) -> Self {
        Self {
            steps,
            reg_src,
            reg_dst,
        }
    }
}

impl Default for OpcodeDescriptor {
    fn default() -> Self {
        Self {
            steps: &EMPTY_STEPS,
            reg_src: 0,
            reg_dst: 0,
        }
    }
}

/// Canonical empty step slice
pub static EMPTY_STEPS: [MicroStep; 0] = [];

/// Default function for serde deserialization
pub fn default_empty_steps() -> &'static [MicroStep] {
    &EMPTY_STEPS
}

/// A recorded bus or internal transaction captured for cycle-exact verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedTransaction {
    Bus {
        is_read: bool,
        is_tas: bool,
        duration: u32,
        fc: u8,
        addr: u32,
        size: BusAccessSize,
        data: u16,
        uds: bool,
        lds: bool,
    },
    Internal {
        duration: u32,
    },
}

/// Instruction retirement and pipeline refill mode when finishing micro-operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MicroRetireMode {
    #[default]
    None,
    StandardPrefetch,
    ScratchPrefetch,
    TargetRefill { target: u32, new_ir: u16 },
}
