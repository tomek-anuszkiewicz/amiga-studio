//! Motorola 68000 Micro-Step State Machine Type Definitions
//!
//! Defines the atomic `MicroStep` descriptor, `StepFn` primitives,
//! ALU function pointers (`AluFn`), size enums, and opcode descriptors.

use crate::state::CpuState;
use crate::Cpu;
use physical_memory::{AddressBus, BusResult};
use serde::{Deserialize, Serialize};

/// Atomic step execution function.
/// Takes the full CPU and memory bus, returning `BusResult<()>` indicating whether
/// the bus cycle completed (`Ready(())`) or stalled on wait states (`WaitState`).
pub type BusFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>;

/// Pure internal ALU operation.
/// Operates strictly on `CpuState` using pre-decoded register indices.
pub type AluFn = fn(state: &mut CpuState, reg_src: u8, reg_dst: u8);

/// Returns data space Function Code (FC 1 for user, FC 5 for supervisor)
#[inline(always)]
pub fn data_fc(state: &CpuState) -> u8 {
    if state.is_supervisor() {
        crate::state::function_code::SUPERVISOR_DATA
    } else {
        crate::state::function_code::USER_DATA
    }
}

/// Returns program space Function Code (FC 2 for user, FC 6 for supervisor)
#[inline(always)]
pub fn prog_fc(state: &CpuState) -> u8 {
    if state.is_supervisor() {
        crate::state::function_code::SUPERVISOR_PROGRAM
    } else {
        crate::state::function_code::USER_PROGRAM
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
    /// Optional function pointer executing the bus cycle (None for idle/finish/pure ALU steps)
    pub bus_fn: Option<BusFn>,
    /// Function pointer for ALU operations (None for pure bus steps)
    pub alu_fn: Option<AluFn>,
    /// Base CPU clocks consumed (4 for bus cycles, 2 for CCK, 0 for instantaneous ALU)
    pub base_clocks: u8,
}

impl PartialEq for MicroStep {
    fn eq(&self, other: &Self) -> bool {
        (match (self.bus_fn, other.bus_fn) {
            (None, None) => true,
            (Some(a), Some(b)) => a as usize == b as usize,
            _ => false,
        }) && (match (self.alu_fn, other.alu_fn) {
            (None, None) => true,
            (Some(a), Some(b)) => a as usize == b as usize,
            _ => false,
        }) && self.base_clocks == other.base_clocks
    }
}

impl Eq for MicroStep {}

impl MicroStep {
    /// Creates an instantaneous ALU micro-step (0 base clocks, no bus cycle)
    #[inline(always)]
    pub const fn alu(alu_fn: AluFn) -> Self {
        Self {
            bus_fn: None,
            alu_fn: Some(alu_fn),
            base_clocks: 0,
        }
    }

    /// Creates a 2-clock Color Clock (CCK) micro-step (1 CCK = 2 CPU clocks)
    #[inline(always)]
    pub const fn cck(bus_fn: BusFn) -> Self {
        Self {
            bus_fn: Some(bus_fn),
            alu_fn: None,
            base_clocks: 2,
        }
    }

    /// Creates a 2-clock CCK idle micro-step (no bus cycle, no ALU action)
    #[inline(always)]
    pub const fn cck_idle() -> Self {
        Self {
            bus_fn: None,
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
