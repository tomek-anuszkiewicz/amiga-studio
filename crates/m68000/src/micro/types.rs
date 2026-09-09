//! Motorola 68000 Micro-Step State Machine Type Definitions
//!
//! Defines the atomic `MicroStep` descriptor, `StepFn` primitives,
//! ALU function pointers (`AluFn`), size enums, and opcode descriptors.

use crate::core::Cpu;
use crate::state::CpuState;
use memory_bus::{AddressBus, BusResult};
use serde::{Deserialize, Serialize};

/// Atomic step execution function.
/// Takes the full CPU and memory bus, returning `BusResult<()>` indicating whether
/// the bus cycle completed (`Ready(())`) or stalled on wait states (`WaitState`).
pub type StepFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>;

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

/// Stateless, cache-dense atomic micro-step descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MicroStep {
    /// Function pointer executing the bus cycle or cycle action
    pub step_fn: StepFn,
    /// Function pointer for ALU operations (None for pure bus steps)
    pub alu_fn: Option<AluFn>,
    /// Base CPU clocks consumed (4 for bus cycles, 0 for instantaneous ALU)
    pub base_clocks: u8,
}

impl PartialEq for MicroStep {
    fn eq(&self, other: &Self) -> bool {
        self.step_fn as usize == other.step_fn as usize
            && (match (self.alu_fn, other.alu_fn) {
                (None, None) => true,
                (Some(a), Some(b)) => a as usize == b as usize,
                _ => false,
            })
            && self.base_clocks == other.base_clocks
    }
}

impl Eq for MicroStep {}

impl MicroStep {
    /// Creates an instantaneous ALU micro-step (0 base clocks)
    #[inline(always)]
    pub const fn alu(alu_fn: AluFn) -> Self {
        Self {
            step_fn: Cpu::step_alu,
            alu_fn: Some(alu_fn),
            base_clocks: 0,
        }
    }

    /// Creates a generic bus cycle micro-step
    #[inline(always)]
    pub const fn bus(step_fn: StepFn, base_clocks: u8) -> Self {
        Self {
            step_fn,
            alu_fn: None,
            base_clocks,
        }
    }

    /// Creates a 2-clock Color Clock (CCK) micro-step (1 CCK = 2 CPU clocks)
    #[inline(always)]
    pub const fn cck(step_fn: StepFn) -> Self {
        Self {
            step_fn,
            alu_fn: None,
            base_clocks: 2,
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
