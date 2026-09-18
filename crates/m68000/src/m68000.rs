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

pub use core::Cpu;
pub use micro::{
    common::VECTOR_ADDRESS_ERROR, CpuMicroState, MicroStep, OpcodeDescriptor, Size,
    OPCODE_DESCRIPTOR_TABLE,
};
pub use state::{
    function_code, vector, CpuState, CCR_ALL, CCR_C, CCR_N, CCR_V, CCR_X, CCR_Z, SR_I_MASK,
    SR_MASK, SR_RESET_DEFAULT, SR_S, SR_T,
};
