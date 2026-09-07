//! Motorola 68000 CPU Emulator Core for the Commodore Amiga 500
//!
//! Provides cycle-exact micro-operations, effective address resolution across
//! all 12 M68000 addressing modes, cycle-exact condition code evaluation,
//! and a 65,536-entry compile-time static direct-dispatch jump table.

pub mod addressing;
pub mod core;
pub mod dispatch_table;
pub mod instructions;
pub mod micro;
pub mod state;

pub use addressing::{AddressingMode, IndexReg, IndexType, Size};
pub use core::{Cpu, StepResult};
pub use dispatch_table::{OpcodeHandler, DISPATCH_TABLE};
pub use micro::{CpuMicroState, MicroRetireMode, RecordedTransaction};
pub use state::CpuState;
