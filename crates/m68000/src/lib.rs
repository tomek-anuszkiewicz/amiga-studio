//! Motorola 68000 CPU Emulator Core for the Commodore Amiga 500
//!
//! Provides cycle-exact micro-step state machine execution, effective address resolution,
//! condition code evaluation, and a 65,536-entry compile-time static opcode descriptor table.

pub mod core;
pub mod instructions;
pub mod micro {
    pub mod common;
    pub mod dispatch_table;
    pub mod ea;
    pub mod engine;
    pub mod step_control;
    pub mod step_execution;
    pub mod types;

    pub use common::*;
    pub use dispatch_table::*;
    pub use ea::*;
    pub use engine::*;
    pub use types::*;
}
pub mod state;

pub use core::{Cpu, StepResult};
pub use micro::{
    CpuMicroState, MicroStep, OpcodeDescriptor, RecordedTransaction, Size, StepFn,
    OPCODE_DESCRIPTOR_TABLE,
};
pub use state::CpuState;
