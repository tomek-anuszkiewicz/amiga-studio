//! Headless Debugger and Inspection Engine for Amiga 500
//!
//! Provides zero-dependency M68000 mini-assembler, trace history ring buffer,
//! execution breakpoints, conditional breakpoints, memory watchpoints, temporal time-travel debugging,
//! headless machine binary loading, and unified execution session controller.

pub mod assembler;
pub mod breakpoints;
pub mod loader;
pub mod session;
pub mod stepping;
pub mod temporal;
pub mod trace;

pub use assembler::assemble_instruction;
pub use breakpoints::{
    BreakpointCondition, BreakpointManager, ConditionOp, ConditionRegister, MemoryWatchpoint,
    PcBreakpoint, WatchAccess,
};
pub use disassembler::{
    disassemble, find_aligned_disassembly_start, format_ea, format_immediate,
    format_movem_reg_list, Disassembly,
};
pub use loader::{inject_binary, DEFAULT_TARGET_ADDRESS};
pub use machine_loop::{
    compute_crc32, A500State, SaveStateError, SaveStateHeader, SAVE_STATE_MAGIC, SAVE_STATE_VERSION,
};
pub use session::DebuggerSession;
pub use stepping::{Debugger, StepMode};
pub use temporal::{TemporalFrame, TemporalHistory, DEFAULT_TEMPORAL_CAPACITY, PAL_FRAME_CCK};
pub use trace::{TraceEntry, TraceRingBuffer};
