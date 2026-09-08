//! Reusable Common Micro-Step Building Blocks
//!
//! Centralizes universal atomic bus steps and retirement sequences shared
//! across all Motorola 68000 instructions.

use super::types::{MicroAction, MicroStep};

// ============================================================================
// Instruction Retirement & Pipeline Refill
// ============================================================================

/// Standard sequential instruction prefetch and retirement (4 clocks / 2 CCKs)
pub const RETIRE_STANDARD: MicroStep = MicroStep {
    action: MicroAction::PrefetchNextOpcodeAndRetire,
    alu_fn: None,
    base_clocks: 4,
};

/// First cycle of taken branch / jump target pipeline refill
pub const READ_TARGET_OPCODE: MicroStep = MicroStep {
    action: MicroAction::BusReadTargetOpcode,
    alu_fn: None,
    base_clocks: 4,
};

/// Second cycle of taken branch / jump target pipeline refill & retirement
pub const PREFETCH_TARGET_RETIRE: MicroStep = MicroStep {
    action: MicroAction::PrefetchTargetAndRetire,
    alu_fn: None,
    base_clocks: 4,
};

// ============================================================================
// Instruction Extension Word Fetching
// ============================================================================

/// Fetches immediate data or displacement word from PC and advances PC += 2
pub const FETCH_EXTENSION: MicroStep = MicroStep {
    action: MicroAction::FetchExtension,
    alu_fn: None,
    base_clocks: 4,
};

// ============================================================================
// Read-Modify-Write (Class 0 RMW) Writeback & Retirement
// ============================================================================

/// Hardware Quirk: Pre-fetches next instruction opcode into scratch before memory write
pub const RMW_PREFETCH_SCRATCH: MicroStep = MicroStep {
    action: MicroAction::BusPrefetchToScratch,
    alu_fn: None,
    base_clocks: 4,
};

/// Writes 8-bit byte result to memory and concludes retirement at CCK2
pub const RMW_WRITE_BYTE_RETIRE: MicroStep = MicroStep {
    action: MicroAction::BusWriteByteAndRetire,
    alu_fn: None,
    base_clocks: 4,
};

/// Writes 16-bit word result to memory and concludes retirement at CCK2
pub const RMW_WRITE_WORD_RETIRE: MicroStep = MicroStep {
    action: MicroAction::BusWriteWordAndRetire,
    alu_fn: None,
    base_clocks: 4,
};

/// Writes high word of 32-bit result to memory (RMW Long cycle 1)
pub const RMW_WRITE_LONG_HIGH: MicroStep = MicroStep {
    action: MicroAction::BusWriteLongHigh,
    alu_fn: None,
    base_clocks: 4,
};

/// Writes low word of 32-bit result to memory + 2 and concludes retirement at CCK2
pub const RMW_WRITE_LONG_LOW_RETIRE: MicroStep = MicroStep {
    action: MicroAction::BusWriteLongLowAndRetire,
    alu_fn: None,
    base_clocks: 4,
};


/// Pops high word of 32-bit address from (SP), stores into scratch[0], and increments SP += 2
pub const POP_STACK_HIGH: MicroStep = MicroStep {
    action: MicroAction::BusPopStackHigh,
    alu_fn: None,
    base_clocks: 4,
};

/// Pops low word of 32-bit address from (SP), stores assembled target into ea_addr, and increments SP += 2
pub const POP_STACK_LOW: MicroStep = MicroStep {
    action: MicroAction::BusPopStackLow,
    alu_fn: None,
    base_clocks: 4,
};

/// Pushes 16-bit high word to -(SP)
pub const PUSH_STACK_HIGH: MicroStep = MicroStep {
    action: MicroAction::BusPushStackHigh,
    alu_fn: None,
    base_clocks: 4,
};

/// Pushes 16-bit low word to -(SP)
pub const PUSH_STACK_LOW: MicroStep = MicroStep {
    action: MicroAction::BusPushStackLow,
    alu_fn: None,
    base_clocks: 4,
};

/// Pushes 16-bit low word to -(SP) and retires with scratch_prefetch (used by PEA)
pub const PUSH_STACK_LOW_RETIRE: MicroStep = MicroStep {
    action: MicroAction::BusPushStackLowAndRetire,
    alu_fn: None,
    base_clocks: 4,
};
