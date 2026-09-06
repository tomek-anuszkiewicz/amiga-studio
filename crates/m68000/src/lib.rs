//! Motorola 68000 CPU Emulator Core for the Commodore Amiga 500
//!
//! Provides cycle-exact micro-operations, effective address resolution across
//! all 12 M68000 addressing modes, and cycle-exact condition code evaluation.

pub mod addressing;
pub mod core;
pub mod instructions;
pub mod state;

pub use addressing::{AddressingMode, IndexReg, IndexType, Size};
pub use core::{Cpu, StepResult};
pub use state::CpuState;
