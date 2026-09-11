//! Headless Debugger and Inspection Engine for Amiga 500
//!
//! Provides zero-dependency M68000 disassembler, trace history ring buffer,
//! execution breakpoints, memory watchpoints, and fine-grained stepping controls.

pub mod breakpoints;
pub mod disassembler;
pub mod stepping;
pub mod trace;

pub use breakpoints::{BreakpointManager, WatchAccess};
pub use disassembler::{disassemble, format_ea, Disassembly};
pub use stepping::{Debugger, StepMode};
pub use trace::{TraceEntry, TraceRingBuffer};
