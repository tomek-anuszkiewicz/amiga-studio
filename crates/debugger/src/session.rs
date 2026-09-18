//! Headless Execution Session & Machine Controller
//!
//! Owns the A500Machine, Debugger, and Temporal History, providing a unified,
//! headless execution controller decoupled from GUI frameworks.

use config::A500Config;
use machine_loop::{A500Machine, A500State, SaveStateError};
use physical_memory::PhysicalMemory;

use crate::loader::inject_binary;
use crate::stepping::Debugger;
use crate::temporal::{TemporalHistory, DEFAULT_TEMPORAL_CAPACITY, PAL_FRAME_CCK};
use m68000::CpuState;

/// Complete headless execution session and machine controller
#[derive(Debug, Clone)]
pub struct DebuggerSession {
    /// The full Amiga 500 machine orchestrating CPU, memory bus, and chips
    pub machine: A500Machine,
    /// Interactive debugger stepping and trace engine
    pub debugger: Debugger,
    /// Circular historical state buffer for time-travel debugging
    pub temporal: TemporalHistory,

    // Execution state
    pub is_running: bool,
    pub instructions_executed: u64,
    pub prev_cpu_state: Option<CpuState>,

    // Diff memory snapshot (256 bytes around prev_hex_base)
    pub prev_hex_bytes: [u8; 256],
    pub prev_hex_base: u32,

    /// Quick Save Slots (slots 1..=5)
    pub quick_slots: [Option<A500State>; 5],
    /// Live CPU state preserved while scrubbing historical frames
    pub live_cpu_state: Option<CpuState>,
}

impl Default for DebuggerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerSession {
    /// Initializes a new DebuggerSession from an explicit A500Config
    pub fn from_config(config: A500Config) -> Self {
        let machine = A500Machine::new(config);

        let mut temporal = TemporalHistory::new(DEFAULT_TEMPORAL_CAPACITY);
        temporal.set_recording(false); // Lean opt-in: recording starts paused by default

        Self {
            machine,
            debugger: Debugger::new(),
            temporal,
            is_running: false,
            instructions_executed: 0,
            prev_cpu_state: None,
            prev_hex_bytes: [0; 256],
            prev_hex_base: 0,
            quick_slots: [None, None, None, None, None],
            live_cpu_state: None,
        }
    }

    /// Initializes a new DebuggerSession with default A500 configuration
    pub fn new() -> Self {
        Self::from_config(A500Config::default())
    }

    /// Direct reference to the machine's physical memory
    #[inline]
    pub fn bus(&self) -> &PhysicalMemory {
        &self.machine.physical_memory
    }

    /// Direct mutable reference to the machine's physical memory
    #[inline]
    pub fn bus_mut(&mut self) -> &mut PhysicalMemory {
        &mut self.machine.physical_memory
    }

    /// Captures a 256-byte snapshot of memory around `base_addr` for diff highlighting
    pub fn capture_memory_snapshot(&mut self, base_addr: u32) {
        for i in 0..256 {
            let addr = base_addr.wrapping_add(i as u32) & 0x00FF_FFFF;
            self.prev_hex_bytes[i] = self.machine.physical_memory.read_byte_debug(addr);
        }
        self.prev_hex_base = base_addr;
    }

    /// Steps exactly 1 M68000 instruction, recording trace and temporal history
    pub fn step_instruction(&mut self) {
        if self.temporal.scrub_cursor.is_some() {
            self.jump_to_live_head();
        }

        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.machine.cpu.state.clone());

        let pc = self.machine.cpu.state.instruction_pc;
        if self.temporal.is_recording() {
            self.temporal.record(
                self.machine.cck,
                pc,
                self.machine.cpu.state.ir,
                self.machine.cpu.state.clone(),
            );
        }
        let (disasm, _) = self
            .debugger
            .disassemble_at(pc, &mut self.machine.physical_memory);
        self.debugger.trace.record(
            self.machine.cck,
            pc,
            self.machine.cpu.state.ir,
            disasm.format_line(),
            self.machine.cpu.state.clone(),
        );

        self.machine.step_instruction();
        self.debugger.current_cck = self.machine.cck;
        self.instructions_executed = self.instructions_executed.saturating_add(1);
    }

    /// Steps exactly 1 Color Clock phase (~280 ns) across the entire machine
    pub fn step_cck(&mut self) {
        if self.temporal.scrub_cursor.is_some() {
            self.jump_to_live_head();
        }

        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.machine.cpu.state.clone());

        let completed = self.machine.step_cck();
        self.debugger.current_cck = self.machine.cck;

        if completed {
            self.instructions_executed = self.instructions_executed.saturating_add(1);
        }
    }

    /// Runs a bounded slice of instructions while free-running
    pub fn run_slice(&mut self, max_instructions: usize) -> usize {
        if !self.is_running {
            return 0;
        }

        if self.temporal.scrub_cursor.is_some() {
            self.jump_to_live_head();
        }

        if self.prev_cpu_state.is_none() {
            self.capture_memory_snapshot(self.prev_hex_base);
            self.prev_cpu_state = Some(self.machine.cpu.state.clone());
        }

        let mut steps = 0;
        while steps < max_instructions {
            let next_pc = self.machine.cpu.state.instruction_pc;
            if self
                .debugger
                .breakpoints
                .check_pc_with_state(next_pc, &self.machine.cpu.state)
            {
                self.is_running = false;
                break;
            }
            if self.machine.cpu.state.halted || self.machine.cpu.state.stopped {
                self.is_running = false;
                break;
            }

            self.step_instruction();
            steps += 1;

            if self.machine.cpu.state.halted || self.machine.cpu.state.stopped {
                self.is_running = false;
                break;
            }
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
            if self.temporal.scrub_cursor.is_none() && self.live_cpu_state.is_none() {
                self.live_cpu_state = Some(self.machine.cpu.state.clone());
            }
            self.machine.cpu.state = frame.state.clone();
            self.temporal.scrub_cursor = Some(index);
        }
    }

    /// Exits history scrub mode and returns to live head
    pub fn jump_to_live_head(&mut self) {
        if let Some(live) = self.live_cpu_state.take() {
            self.machine.cpu.state = live;
        }
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
        self.machine.reset_cold();
        self.temporal.clear();
        self.debugger.trace.clear();
        self.debugger.current_cck = 0;
        self.instructions_executed = 0;
        self.prev_cpu_state = None;
        self.live_cpu_state = None;
    }

    /// Warm-resets the A500 machine
    pub fn reset_warm(&mut self) {
        self.is_running = false;
        self.machine.reset_warm();
        self.prev_cpu_state = None;
        self.live_cpu_state = None;
    }

    /// Loads binary data into memory and optionally sets PC & primes prefetch
    pub fn load_binary(&mut self, target_addr: u32, data: &[u8], auto_prime: bool) -> usize {
        inject_binary(
            &mut self.machine.cpu,
            &mut self.machine.physical_memory,
            target_addr,
            data,
            auto_prime,
        )
    }

    /// Saves the current machine state into an `A500State` snapshot
    pub fn save_state(&self) -> A500State {
        self.machine.save_state()
    }

    /// Saves the machine state in self-contained mode (embedding Kickstart ROM)
    pub fn save_state_self_contained(&self) -> A500State {
        self.machine.save_state_self_contained()
    }

    /// Restores machine state from an `A500State` snapshot and synchronizes debugger tracking
    pub fn load_state(&mut self, state: &A500State) -> Result<(), SaveStateError> {
        self.machine.load_state(state)?;

        // Synchronize debugger telemetry and execution tracking
        self.debugger.current_cck = self.machine.cck;
        self.prev_cpu_state = Some(self.machine.cpu.state.clone());
        self.capture_memory_snapshot(self.prev_hex_base);
        self.jump_to_live_head();
        self.is_running = false;

        Ok(())
    }

    /// Serializes machine state to pretty-printed JSON
    pub fn save_state_to_json(&self) -> Result<String, SaveStateError> {
        self.save_state().to_json_pretty()
    }

    /// Deserializes machine state from JSON
    pub fn load_state_from_json(&mut self, json_str: &str) -> Result<(), SaveStateError> {
        let state = A500State::from_json(json_str)?;
        self.load_state(&state)
    }

    /// Saves machine state directly to a file (compressed or JSON)
    pub fn save_state_to_file(
        &self,
        path: impl AsRef<std::path::Path>,
        self_contained: bool,
    ) -> Result<(), SaveStateError> {
        self.machine.save_state_to_file(path, self_contained)
    }

    /// Loads machine state directly from a file
    pub fn load_state_from_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), SaveStateError> {
        let state = A500State::load_from_file(path)?;
        self.load_state(&state)
    }

    /// Saves machine snapshot to an in-memory quick slot (1..=5)
    pub fn save_quick_slot(&mut self, slot: usize) -> Result<(), SaveStateError> {
        if slot == 0 || slot > 5 {
            return Err(SaveStateError::CorruptedData(format!(
                "Invalid quick slot index: {slot} (must be 1..=5)"
            )));
        }
        self.quick_slots[slot - 1] = Some(self.save_state());
        Ok(())
    }

    /// Loads machine snapshot from an in-memory quick slot (1..=5)
    pub fn load_quick_slot(&mut self, slot: usize) -> Result<(), SaveStateError> {
        if slot == 0 || slot > 5 {
            return Err(SaveStateError::CorruptedData(format!(
                "Invalid quick slot index: {slot} (must be 1..=5)"
            )));
        }
        let state = self.quick_slots[slot - 1]
            .clone()
            .ok_or_else(|| SaveStateError::CorruptedData(format!("Quick slot {slot} is empty")))?;
        self.load_state(&state)
    }

    /// Returns true if the specified quick slot (1..=5) is populated
    #[inline]
    pub fn has_quick_slot(&self, slot: usize) -> bool {
        if slot == 0 || slot > 5 {
            false
        } else {
            self.quick_slots[slot - 1].is_some()
        }
    }
}
