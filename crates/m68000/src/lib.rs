//! Motorola 68000 CPU Emulator Core for the Commodore Amiga 500
//!
//! Provides cycle-exact micro-step state machine execution, effective address resolution,
//! condition code evaluation, and a 65,536-entry compile-time static opcode descriptor table.

pub mod core;
pub mod instructions;
pub mod micro;
pub mod state;

pub use core::{Cpu, StepResult};
pub use micro::{
    CpuMicroState, MicroAction, MicroRetireMode, MicroStep, OpcodeDescriptor,
    RecordedTransaction, Size, OPCODE_DESCRIPTOR_TABLE,
};
pub use state::CpuState;
