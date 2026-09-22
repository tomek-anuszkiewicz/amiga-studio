//! Motorola 68000 Micro-Step State Machine Execution Engine
//!
//! Models cycle-exact 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock
//! CPU bus cycle, driving atomic MicroSteps directly from pre-compiled slices.

use super::types::{default_empty_steps, MicroStep, OpcodeDescriptor, EMPTY_STEPS};
use serde::{Deserialize, Serialize};

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Instruction Register Capture prefetch latch (68000 IRC)
    #[serde(default)]
    pub irc: u16,
    /// Step index within the current instruction's micro-operation sequence
    pub micro_step: u16,
    /// Intermediate high-word buffer for 32-bit address/immediate assembly
    #[serde(default)]
    pub ea_high: u32,
    /// Active 16-bit register mask for MOVEM block transfers
    #[serde(default)]
    pub movem_mask: u16,
    /// Transfer progress state for MOVEM block transfers
    #[serde(default)]
    pub movem_state: u16,
    /// Clocks remaining for the active micro-step (0 when completed or between steps)
    #[serde(default)]
    pub clocks_remaining: u16,

    // --- Micro-Step State Machine Fields ---
    /// Decoded source operand buffer for ALU operations
    #[serde(default)]
    pub source: u32,
    /// Decoded destination operand buffer and ALU output result buffer
    #[serde(default)]
    pub destination: u32,
    /// Resolved effective memory address for operands or branch/jump targets
    #[serde(default)]
    pub ea_addr: u32,
    /// Dual Staging Register 1 (X1 / Source EA / High-word / Low-word split EA)
    #[serde(default)]
    pub addr1: u32,
    /// Dual Staging Register 2 (X2 / Destination EA / High-word / Low-word split EA)
    #[serde(default)]
    pub addr2: u32,
    /// Pre-decoded source register index (0..7 for Dn/An)
    #[serde(default)]
    pub reg_src: u8,
    /// Pre-decoded destination register index (0..7 for Dn/An)
    #[serde(default)]
    pub reg_dst: u8,
    /// Cached pointer to the active opcode's slice of MicroSteps
    #[serde(skip, default = "default_empty_steps")]
    pub current_steps: &'static [MicroStep],
    /// Indicates whether instruction retirement must perform a branch/jump target refill
    #[serde(default)]
    pub target_refill: bool,
    /// Indicates whether prefetch pipeline has already retired into IR during microcode execution
    #[serde(default)]
    pub prefetch_retired: bool,
    /// Fault address for Group 0 Address Error / Bus Error exception
    #[serde(default)]
    pub fault_addr: u32,
    /// Internal Information Word for Group 0 exception frame
    #[serde(default)]
    pub info_word: u16,
    /// Base supervisor stack pointer at start of exception frame stacking
    #[serde(default)]
    pub ssp_base: u32,
}

impl PartialEq for CpuMicroState {
    fn eq(&self, other: &Self) -> bool {
        self.irc == other.irc
            && self.micro_step == other.micro_step
            && self.ea_high == other.ea_high
            && self.movem_mask == other.movem_mask
            && self.movem_state == other.movem_state
            && self.clocks_remaining == other.clocks_remaining
            && self.source == other.source
            && self.destination == other.destination
            && self.ea_addr == other.ea_addr
            && self.addr1 == other.addr1
            && self.addr2 == other.addr2
            && self.reg_src == other.reg_src
            && self.reg_dst == other.reg_dst
            && self.target_refill == other.target_refill
            && self.prefetch_retired == other.prefetch_retired
            && self.fault_addr == other.fault_addr
            && self.info_word == other.info_word
            && self.ssp_base == other.ssp_base
    }
}

impl Eq for CpuMicroState {}

impl Default for CpuMicroState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuMicroState {
    /// Creates a new CPU micro-state
    pub fn new() -> Self {
        Self {
            irc: 0,
            micro_step: 0,
            ea_high: 0,
            movem_mask: 0,
            movem_state: 0,
            clocks_remaining: 0,
            source: 0,
            destination: 0,
            ea_addr: 0,
            addr1: 0,
            addr2: 0,
            reg_src: 0,
            reg_dst: 0,
            current_steps: &EMPTY_STEPS,
            target_refill: false,
            prefetch_retired: false,
            fault_addr: 0,
            info_word: 0,
            ssp_base: 0,
        }
    }

    /// Resets the micro-state machine to initial power-on / reset state
    pub fn reset(&mut self) {
        self.irc = 0;
        self.micro_step = 0;
        self.ea_high = 0;
        self.movem_mask = 0;
        self.movem_state = 0;
        self.clocks_remaining = 0;
        self.source = 0;
        self.destination = 0;
        self.ea_addr = 0;
        self.addr1 = 0;
        self.addr2 = 0;
        self.reg_src = 0;
        self.reg_dst = 0;
        self.current_steps = &EMPTY_STEPS;
        self.target_refill = false;
        self.prefetch_retired = false;
        self.fault_addr = 0;
        self.info_word = 0;
        self.ssp_base = 0;
    }

    /// Initializes active micro-steps for a new instruction
    #[inline(always)]
    pub fn initiate_instruction(&mut self, desc: &OpcodeDescriptor) {
        self.current_steps = desc.steps;
        self.reg_src = desc.reg_src;
        self.reg_dst = desc.reg_dst;
        self.micro_step = 0;
        self.clocks_remaining = 0;
    }
}
