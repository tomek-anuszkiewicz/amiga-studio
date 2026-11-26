//! Headless Execution Session & Machine Controller
//!
//! Owns the A500Machine, Debugger, and Temporal History, providing a unified,
//! headless execution controller decoupled from GUI frameworks.

use config::A500Config;
use machine_loop::{A500Machine, A500State};
use physical_memory::PhysicalMemory;
use save_state::{MediaSources, SaveState, SaveStateError, SourceImage};

use crate::loader::inject_binary;
use crate::stepping::Debugger;
use crate::temporal::{TemporalHistory, DEFAULT_TEMPORAL_CAPACITY, PAL_FRAME_CCK};
use cpu::CpuState;

pub const MEMORY_DIFF_BYTES: usize = 256;
/// Bounded host work, slightly longer than a normal PAL field.
pub const MAX_FRAME_SLICE_CCK: usize = 72_000;

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
    debugger_enabled: bool,
    pause_at_frame: bool,
    stepping_frame: bool,
    enable_debugger_on_pause: bool,
    unpublished_instructions: u64,
    frame_boundary_cck: Option<u64>,
    instruction_started: bool,
    instruction_recorded: bool,
    pub instructions_executed: u64,
    pub prev_cpu_state: Option<CpuState>,

    // Diff memory snapshot (256 bytes around prev_hex_base)
    pub prev_hex_bytes: [u8; MEMORY_DIFF_BYTES],
    pub prev_hex_base: u32,

    pub(crate) file_sources: MediaSources,
    /// Full in-memory snapshot and its original images for subsequent file export.
    quicksave: Option<(Box<A500State>, MediaSources)>,
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
            debugger_enabled: true,
            pause_at_frame: false,
            stepping_frame: false,
            enable_debugger_on_pause: false,
            unpublished_instructions: 0,
            frame_boundary_cck: None,
            instruction_started: false,
            instruction_recorded: false,
            instructions_executed: 0,
            prev_cpu_state: None,
            prev_hex_bytes: [0; MEMORY_DIFF_BYTES],
            prev_hex_base: 0,
            file_sources: MediaSources::default(),
            quicksave: None,
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

    /// Connects inspection to execution, or bypasses it completely for Game view.
    pub fn set_debugger_enabled(&mut self, enabled: bool) {
        self.enable_debugger_on_pause = enabled && !self.debugger_enabled && self.is_running;
        if self.enable_debugger_on_pause {
            self.request_pause();
            return;
        }
        if self.debugger_enabled == enabled {
            return;
        }
        self.jump_to_live_head();
        self.debugger_enabled = enabled;
        self.prev_cpu_state = None;
        if enabled {
            self.synchronize_inspection();
        }
    }

    /// Continuous execution includes an ordinary pause waiting for its field boundary.
    pub fn continuous_run(&self) -> bool {
        self.is_running && !self.stepping_frame
    }

    fn synchronize_inspection(&mut self) {
        self.debugger.current_cck = self.machine.cck;
        self.instructions_executed = self
            .instructions_executed
            .saturating_add(self.unpublished_instructions);
        self.unpublished_instructions = 0;
        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.machine.cpu.state.clone());
    }

    fn finish_run(&mut self) {
        let continuous = !self.stepping_frame;
        self.is_running = false;
        self.pause_at_frame = false;
        self.stepping_frame = false;
        if self.enable_debugger_on_pause {
            self.debugger_enabled = true;
            self.enable_debugger_on_pause = false;
        }
        if continuous && self.debugger_enabled {
            self.synchronize_inspection();
        }
    }

    /// Captures a 256-byte snapshot of memory around `base_addr` for diff highlighting
    pub fn capture_memory_snapshot(&mut self, base_addr: u32) {
        for i in 0..MEMORY_DIFF_BYTES {
            let addr = base_addr.wrapping_add(i as u32) & physical_memory::MASK_PHYSICAL_ADDRESS;
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

        self.record_instruction();
        self.machine.step_instruction();
        self.instruction_started = false;
        self.instruction_recorded = false;
        self.debugger.current_cck = self.machine.cck;
        self.instructions_executed = self.instructions_executed.saturating_add(1);
    }

    /// Restarts breakpoint and history checks after a host edit replaces CPU execution.
    pub fn invalidate_instruction_tracking(&mut self) {
        self.instruction_started = false;
        self.instruction_recorded = false;
    }

    fn record_instruction(&mut self) {
        if self.instruction_recorded {
            return;
        }
        self.instruction_recorded = true;
        let pc = self.machine.cpu.state.instruction_pc();
        if self.temporal.is_recording() {
            self.temporal.record(
                self.machine.cck,
                pc,
                self.machine.cpu.state.ir(),
                self.machine.cpu.state.clone(),
            );
        }
        let (disasm, _) = self
            .debugger
            .disassemble_at(pc, &mut self.machine.physical_memory);
        self.debugger.trace.record(
            self.machine.cck,
            pc,
            self.machine.cpu.state.ir(),
            disasm.format_line(),
            self.machine.cpu.state.clone(),
        );
    }

    /// Steps exactly 1 Color Clock phase (~280 ns) across the entire machine
    pub fn step_cck(&mut self) {
        if self.temporal.scrub_cursor.is_some() {
            self.jump_to_live_head();
        }

        self.capture_memory_snapshot(self.prev_hex_base);
        self.prev_cpu_state = Some(self.machine.cpu.state.clone());

        self.record_instruction();
        self.advance_cck();
    }

    fn advance_cck(&mut self) {
        let frame = self.machine.denise.frame_builder.frame_number;
        let completed = self.machine.step_cck();
        self.debugger.current_cck = self.machine.cck;
        self.instruction_started = !completed;
        if completed {
            self.instruction_recorded = false;
        }
        if completed {
            self.instructions_executed = self.instructions_executed.saturating_add(1);
        }
        if self.machine.denise.frame_builder.frame_number != frame {
            self.frame_boundary_cck = Some(self.machine.cck);
        }
    }

    /// Runs at most `max_cck`, stopping at the next completed video field.
    /// Returns the number of retired instructions; a field may end mid-instruction.
    pub fn run_frame_slice(&mut self, max_cck: usize) -> usize {
        if !self.is_running {
            return 0;
        }
        let diagnostics = self.debugger_enabled && self.stepping_frame;
        let check_breakpoints = self.debugger_enabled
            && self
                .debugger
                .breakpoints
                .pc_breakpoints
                .iter()
                .any(|bp| bp.enabled);
        let initial_frame = self.machine.denise.frame_builder.frame_number;
        let mut retired = 0;
        for _ in 0..max_cck {
            if self.machine.cpu.state.is_halted() {
                self.is_running = false;
                break;
            }
            // STOP waits for an interrupt while the rest of the machine keeps running.
            let needs_debugger = (check_breakpoints && !self.instruction_started)
                || (diagnostics && !self.instruction_recorded);
            if needs_debugger && !self.machine.cpu.state.is_stopped() {
                if !self.instruction_started
                    && check_breakpoints
                    && self.debugger.breakpoints.check_pc_with_state(
                        self.machine.cpu.state.instruction_pc(),
                        &self.machine.cpu.state,
                    )
                {
                    self.is_running = false;
                    break;
                }
                if diagnostics && !self.instruction_recorded {
                    self.capture_memory_snapshot(self.prev_hex_base);
                    self.prev_cpu_state = Some(self.machine.cpu.state.clone());
                    self.record_instruction();
                }
            }
            let previous_cck = self.machine.cck;
            let completed = self.machine.step_cck();
            if self.machine.cck.wrapping_sub(previous_cck) != 1 {
                // A motherboard keyboard reset starts a new clock timeline.
                self.frame_boundary_cck = None;
                self.invalidate_instruction_tracking();
                break;
            }
            self.instruction_started = !completed;
            if completed {
                self.instruction_recorded = false;
            }
            retired += usize::from(completed);
            if self.debugger_enabled && self.machine.cpu.state.is_halted() {
                self.is_running = false;
                break;
            }
            if self.machine.denise.frame_builder.frame_number != initial_frame {
                self.frame_boundary_cck = Some(self.machine.cck);
                if self.pause_at_frame {
                    self.is_running = false;
                }
                break;
            }
        }
        // Bookkeeping is independent of diagnostic collection, including Game intervals.
        self.unpublished_instructions =
            self.unpublished_instructions.saturating_add(retired as u64);
        if self.debugger_enabled {
            self.instructions_executed = self
                .instructions_executed
                .saturating_add(self.unpublished_instructions);
            self.unpublished_instructions = 0;
            self.debugger.current_cck = self.machine.cck;
        }
        if !self.is_running {
            self.finish_run();
        }
        retired
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
    fn step_backward_n(&mut self, delta: usize) {
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
            self.machine.cpu.restore_state(frame.state.clone());
            self.temporal.scrub_cursor = Some(index);
        }
    }

    /// Exits history scrub mode and returns to live head
    pub fn jump_to_live_head(&mut self) {
        if let Some(live) = self.live_cpu_state.take() {
            self.machine.cpu.restore_state(live);
        }
        self.temporal.scrub_cursor = None;
    }

    /// Toggles free-running continuous execution
    pub fn toggle_run(&mut self) {
        if self.is_running {
            self.request_pause();
        } else {
            self.pause_at_frame = false;
            self.stepping_frame = false;
            self.is_running = true;
            self.jump_to_live_head();
        }
    }

    /// Finishes the current field before pausing, unless already at its boundary.
    pub fn request_pause(&mut self) {
        if self.at_frame_boundary() {
            self.finish_run();
        }
        self.pause_at_frame = self.is_running;
    }

    /// Whether execution is at the last acknowledged completed video field.
    pub fn at_frame_boundary(&self) -> bool {
        self.frame_boundary_cck == Some(self.machine.cck)
    }

    pub fn pause_pending(&self) -> bool {
        self.is_running && self.pause_at_frame
    }

    /// Starts one field step, executed through bounded `run_frame_slice` calls.
    pub fn step_frame(&mut self) {
        self.jump_to_live_head();
        self.is_running = true;
        self.pause_at_frame = true;
        self.stepping_frame = true;
    }

    /// Resets the A500 machine
    pub fn reset(&mut self) {
        self.is_running = false;
        self.pause_at_frame = false;
        self.stepping_frame = false;
        self.enable_debugger_on_pause = false;
        self.frame_boundary_cck = None;
        self.instruction_started = false;
        self.instruction_recorded = false;
        self.machine.reset();
        self.temporal.clear();
        self.debugger.trace.clear();
        self.debugger.current_cck = 0;
        self.instructions_executed = 0;
        self.unpublished_instructions = 0;
        self.prev_cpu_state = None;
        self.live_cpu_state = None;
    }

    /// Warm-resets the A500 machine
    pub fn reset_warm(&mut self) {
        self.is_running = false;
        self.pause_at_frame = false;
        self.stepping_frame = false;
        self.enable_debugger_on_pause = false;
        self.frame_boundary_cck = None;
        self.instruction_started = false;
        self.instruction_recorded = false;
        self.machine.reset_warm();
        self.prev_cpu_state = None;
        self.live_cpu_state = None;
    }

    /// Loads binary data into memory and optionally sets PC & primes prefetch
    pub fn load_binary(
        &mut self,
        target_addr: u32,
        data: &[u8],
        auto_prime: bool,
    ) -> Result<usize, physical_memory::KickstartSizeError> {
        let result = inject_binary(
            &mut self.machine.cpu,
            &mut self.machine.physical_memory,
            target_addr,
            data,
            auto_prime,
        );
        if result.is_ok() {
            if auto_prime {
                self.instruction_started = false;
                self.instruction_recorded = false;
            }
            if target_addr & physical_memory::MASK_PHYSICAL_ADDRESS
                == config::address_map::KICKSTART_BASE
                && self.machine.physical_memory.kickstart_rom() == data
            {
                self.file_sources.rom = Some(SourceImage {
                    name: None,
                    bytes: data.to_vec(),
                });
            }
        }
        result
    }

    /// Installs a complete Kickstart image and boots its reset vectors, paused.
    /// Validation precedes every mutation, including reset and execution state.
    pub fn load_rom(&mut self, data: &[u8]) -> Result<(), physical_memory::KickstartSizeError> {
        physical_memory::validate_kickstart_size(data.len())?;
        self.load_named_rom(SourceImage {
            name: None,
            bytes: data.to_vec(),
        })
    }

    pub fn load_named_rom(
        &mut self,
        source: SourceImage,
    ) -> Result<(), physical_memory::KickstartSizeError> {
        self.machine
            .physical_memory
            .write_bytes_debug(config::address_map::KICKSTART_BASE, &source.bytes)?;
        self.file_sources.rom = Some(source);
        self.reset();
        Ok(())
    }

    pub fn file_sources(&self) -> &MediaSources {
        &self.file_sources
    }

    pub fn name_file_sources(
        &mut self,
        rom: Option<String>,
        disks: [Option<String>; config::FLOPPY_DRIVE_COUNT],
    ) {
        if let Some(source) = &mut self.file_sources.rom {
            source.name = rom;
        }
        for (source, name) in self.file_sources.disks.iter_mut().zip(disks) {
            if let Some(source) = source {
                source.name = name;
            }
        }
    }

    /// Saves the current machine state into an `A500State` snapshot
    pub fn save_state(&self) -> A500State {
        self.machine.save_state()
    }

    /// Restores machine state from an `A500State` snapshot and synchronizes debugger tracking
    pub fn load_state(&mut self, state: &A500State) {
        self.machine.load_state(state);
        self.file_sources = MediaSources::default();
        self.finish_state_load();
    }

    fn finish_state_load(&mut self) {
        self.pause_at_frame = false;
        self.stepping_frame = false;
        self.enable_debugger_on_pause = false;
        self.frame_boundary_cck = None;
        self.instruction_started = false;
        self.instruction_recorded = false;

        // Synchronize debugger telemetry and execution tracking
        self.debugger.current_cck = self.machine.cck;
        self.live_cpu_state = None;
        self.temporal.scrub_cursor = None;
        self.prev_cpu_state = None;
        if self.debugger_enabled {
            self.prev_cpu_state = Some(self.machine.cpu.state.clone());
            self.capture_memory_snapshot(self.prev_hex_base);
        }
        self.is_running = false;
    }

    /// Serializes machine state to pretty-printed JSON
    pub fn save_state_to_json(&self) -> Result<String, SaveStateError> {
        self.save_state_to_json_with_sources(&self.file_sources)
    }

    /// Exports using explicitly supplied bytes, including a previously patched ROM.
    pub fn save_state_to_json_with_sources(
        &self,
        sources: &MediaSources,
    ) -> Result<String, SaveStateError> {
        SaveState::capture(&self.machine, sources)?.to_json_pretty()
    }

    /// Restores a JSON snapshot using matching images mounted in the current session.
    pub fn load_state_from_json(&mut self, json_str: &str) -> Result<(), SaveStateError> {
        self.load_state_from_json_with_sources(json_str, self.file_sources.clone())
    }

    /// Restores a JSON snapshot by matching supplied disk images to saved drives.
    pub fn load_state_from_json_with_sources(
        &mut self,
        json: &str,
        mut sources: MediaSources,
    ) -> Result<(), SaveStateError> {
        let saved = SaveState::from_json(json)?;
        sources.disks = std::array::from_fn(|drive| {
            if !saved.disk_inserted(drive) {
                return None;
            }
            let (earlier, from_target) = sources.disks.split_at(drive);
            from_target
                .iter()
                .chain(earlier)
                .flatten()
                .find(|image| saved.disk_matches(drive, &image.bytes))
                // Retain the codec's mismatch error for an incorrect image in this drive.
                .or(sources.disks[drive].as_ref())
                .cloned()
        });
        if saved.synthetic_rom() {
            sources.rom = None;
        }
        let state = saved.restore(&sources)?;
        self.load_file_state(state, sources);
        Ok(())
    }

    /// Installs an already resolved file snapshot and keeps its verified media.
    pub fn load_file_state(&mut self, state: Box<A500State>, sources: MediaSources) {
        self.machine.load_owned_state(*state);
        self.file_sources = sources;
        self.finish_state_load();
    }

    /// Replace the single in-memory quicksave with the current machine state.
    pub fn quick_save(&mut self) {
        self.quicksave = Some((Box::new(self.save_state()), self.file_sources.clone()));
    }

    /// Inspect the stored state without changing the running machine.
    pub fn quicksave(&self) -> Option<&A500State> {
        self.quicksave.as_ref().map(|(state, _)| state.as_ref())
    }

    /// Restore the in-memory quicksave. The caller controls whether to resume.
    pub fn quick_load(&mut self) -> Result<(), SaveStateError> {
        let (state, sources) = self
            .quicksave
            .as_ref()
            .ok_or_else(|| SaveStateError::CorruptedData("No quicksave is available".into()))?;
        self.machine.load_state(state);
        self.file_sources = sources.clone();
        self.finish_state_load();
        Ok(())
    }
}
