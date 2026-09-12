//! Headless Execution Session & Machine Controller
//!
//! Owns the CPU, MemoryBus, Debugger, and Temporal History, providing a unified,
//! headless execution controller decoupled from GUI frameworks.

use crate::loader::inject_binary;
use crate::stepping::Debugger;
use crate::temporal::{TemporalHistory, DEFAULT_TEMPORAL_CAPACITY, PAL_FRAME_CCK};
use m68000::{Cpu, CpuState};
use memory_bus::MemoryBus;

/// Complete headless execution session and machine controller
#[derive(Debug, Clone)]
pub struct DebuggerSession {
    pub cpu: Cpu,
    pub bus: MemoryBus,
    pub debugger: Debugger,
    pub temporal: TemporalHistory,

    // Execution state
    pub is_running: bool,
    pub instructions_executed: u64,
    pub prev_cpu_state: Option<CpuState>,

    // Diff memory snapshot (256 bytes around prev_hex_base)
    pub prev_hex_bytes: [u8; 256],
    pub prev_hex_base: u32,
}

impl Default for DebuggerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerSession {
    /// Initializes a new DebuggerSession with reset CPU and default temporal capacity
    pub fn new() -> Self {
        let mut bus = MemoryBus::new();
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);

        let mut temporal = TemporalHistory::new(DEFAULT_TEMPORAL_CAPACITY);
        temporal.set_recording(false); // Lean opt-in: recording starts paused by default

        Self {
            cpu,
            bus,
            debugger: Debugger::new(),
            temporal,
            is_running: false,
            instructions_executed: 0,
            prev_cpu_state: None,
            prev_hex_bytes: [0; 256],
            prev_hex_base: 0,
        }
    }

    /// Captures a 256-byte snapshot of memory around `base_addr` for diff highlighting
    pub fn capture_memory_snapshot(&mut self, base_addr: u32) {
        for i in 0..256 {
            let addr = base_addr.wrapping_add(i as u32) & 0x00FF_FFFF;
            self.prev_hex_bytes[i] = self.bus.read_byte_debug(addr);
        }
        self.prev_hex_base = base_addr;
    }

    /// Steps exactly 1 M68000 instruction, recording trace and temporal history
    pub fn step_instruction(&mut self) {
        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.cpu.state.clone());

        let pc = self.cpu.state.instruction_pc;
        self.temporal.record(
            self.debugger.current_cck,
            pc,
            self.cpu.state.ir,
            self.cpu.state.clone(),
        );

        let clocks = self.debugger.step_instruction(&mut self.cpu, &mut self.bus);
        self.instructions_executed = self.instructions_executed.saturating_add(1);

        // Update MemoryBus RTC / timers
        self.bus.step_cck((clocks as u64) / 2);
    }

    /// Steps exactly 1 Color Clock phase (2 CPU clocks)
    pub fn step_cck(&mut self) {
        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.cpu.state.clone());
        let completed = self.cpu.step_cck(&mut self.bus);
        self.debugger.current_cck = self.debugger.current_cck.wrapping_add(1);
        self.bus.step_cck(1);

        if completed {
            self.instructions_executed = self.instructions_executed.saturating_add(1);
        }
    }

    /// Runs a bounded slice of instructions while free-running
    pub fn run_slice(&mut self, max_instructions: usize) -> usize {
        if !self.is_running {
            return 0;
        }

        if self.prev_cpu_state.is_none() {
            self.capture_memory_snapshot(self.prev_hex_base);
            self.prev_cpu_state = Some(self.cpu.state.clone());
        }

        let steps = self.debugger.run_until_breakpoint_with_temporal(
            &mut self.cpu,
            &mut self.bus,
            &mut self.temporal,
            max_instructions,
        );
        self.instructions_executed = self.instructions_executed.saturating_add(steps as u64);

        if self.cpu.state.halted || self.cpu.state.stopped {
            self.is_running = false;
        }

        steps
    }

    /// Rewinds 1 step backward in execution history
    pub fn step_backward(&mut self) {
        self.step_backward_n(1);
    }

    /// Advances 1 step forward along recorded history towards live head
    pub fn step_forward(&mut self) {
        self.step_forward_n(1);
    }

    /// Rewinds by N steps in execution history
    pub fn step_backward_n(&mut self, delta: usize) {
        if let Some(target_idx) = self.temporal.step_back_n(delta) {
            self.scrub_to_frame(target_idx);
        }
    }

    /// Advances by N steps in execution history toward live head
    pub fn step_forward_n(&mut self, delta: usize) {
        if let Some(target_idx) = self.temporal.step_forward_n(delta) {
            self.scrub_to_frame(target_idx);
        } else {
            self.jump_to_live_head();
        }
    }

    /// Rewinds by ~1 PAL video frame (~70,824 CCKs)
    pub fn step_backward_frame(&mut self) {
        if let Some(target_idx) = self.temporal.step_frame_back(PAL_FRAME_CCK) {
            self.scrub_to_frame(target_idx);
        }
    }

    /// Advances forward by ~1 PAL video frame (~70,824 CCKs)
    pub fn step_forward_frame(&mut self) {
        if let Some(target_idx) = self.temporal.step_frame_forward(PAL_FRAME_CCK) {
            self.scrub_to_frame(target_idx);
        } else {
            self.jump_to_live_head();
        }
    }

    /// Scrubs directly to the frame closest to `target_cck`
    pub fn jump_to_cck(&mut self, target_cck: u64) {
        if let Some(target_idx) = self.temporal.find_closest_cck(target_cck) {
            self.scrub_to_frame(target_idx);
        }
    }

    /// Scrubs to a historical snapshot in the ring buffer
    pub fn scrub_to_frame(&mut self, index: usize) {
        if let Some(frame) = self.temporal.get_chronological(index) {
            self.cpu.state = frame.state.clone();
            self.temporal.scrub_cursor = Some(index);
        }
    }

    /// Exits history scrub mode and returns to live head
    pub fn jump_to_live_head(&mut self) {
        self.temporal.scrub_cursor = None;
    }

    /// Toggles free-running continuous execution
    pub fn toggle_run(&mut self) {
        self.is_running = !self.is_running;
        if self.is_running {
            self.jump_to_live_head();
        }
    }

    /// Cold-resets the A500 machine
    pub fn reset_cold(&mut self) {
        self.is_running = false;
        self.bus.reset_cold();
        self.cpu.reset(&mut self.bus);
        self.temporal.clear();
        self.debugger.trace.clear();
        self.debugger.current_cck = 0;
        self.instructions_executed = 0;
        self.prev_cpu_state = None;
    }

    /// Warm-resets the A500 machine
    pub fn reset_warm(&mut self) {
        self.is_running = false;
        self.bus.reset_warm();
        self.cpu.reset(&mut self.bus);
        self.prev_cpu_state = None;
    }

    /// Loads binary data into memory and optionally sets PC & primes prefetch
    pub fn load_binary(&mut self, target_addr: u32, data: &[u8], auto_prime: bool) -> usize {
        inject_binary(&mut self.cpu, &mut self.bus, target_addr, data, auto_prime)
    }
}
