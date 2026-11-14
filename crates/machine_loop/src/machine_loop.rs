//! Amiga 500 Main Machine Loop & Subsystem Coordination
//!
//! Tier 0 top-level machine chassis that owns and coordinates all peer subsystems,
//! custom chips, coprocessors, and peripheral devices in a flat, decoupled structure.

pub use agnus;
pub use cia;
pub use config;
pub use cpu;
pub use denise;
pub use floppy;
pub use game_ports;
pub use keyboard;
pub use memory_bus;
pub use memory_bus::MemoryBus;
pub use paula;
pub use physical_memory;
pub use physical_memory::{AddressBus, BusResult, PhysicalMemory};
pub use rtc;
pub mod state;
pub use state::A500State;
mod signals;

use config::A500Config;
use cpu::Cpu;

/// Top-level Amiga 500 machine struct orchestrating all subsystems
#[derive(Debug, Clone)]
pub struct A500Machine {
    /// Captured keyboard reset level awaiting the next CCK.
    pub keyboard_reset_asserted: bool,
    /// Active hardware configuration
    pub config: A500Config,
    /// Motorola 68000 cycle-exact CPU core
    pub cpu: Cpu,
    /// 24-bit physical storage and RAM/ROM buffers
    pub physical_memory: PhysicalMemory,
    /// Monotonic master 64-bit Color Clock counter (CCK)
    pub cck: u64,
    /// Real-Time Clock (OKI MSM6242B) at $DC0000..$DC003F
    pub rtc: rtc::RtcMsm6242b,

    // --- Custom Chipsets ---
    /// Agnus (beam counters, copper, blitter, dma, chip RAM bus arbitration)
    pub agnus: agnus::Agnus,
    /// Denise (video control, sprites, frame builder, palette, collision registers)
    pub denise: denise::Denise,
    /// Paula (audio, serial UART, interrupt request / enable arbitration)
    pub paula: paula::Paula,
    /// CIA-A (timers, TOD, port A/B, keyboard interface, overlay)
    pub cia_a: cia::Cia,
    /// CIA-B (timers, TOD, port A/B, parallel handshakes)
    pub cia_b: cia::Cia,

    // --- Peripherals & Controller Devices ---
    /// Paula 3.5" DD floppy disk drive controller & mechanics
    pub floppy: floppy::FloppyController,
    /// MOS 6500/1 keyboard microcontroller with reset logic
    pub keyboard: keyboard::Keyboard,
    /// Amiga dual controller game ports (Port 1 Mouse / Port 2 Joystick)
    pub game_ports: game_ports::GamePorts,
}

impl A500Machine {
    /// Drains generated 44.1 kHz stereo PCM for a host consumer.
    pub fn read_audio_samples(&mut self, destination: &mut [paula::audio::StereoSample]) -> usize {
        self.paula.audio.read_samples(destination)
    }

    /// Constructs a zero-cost stack-allocated MemoryBus router spanning all subsystems
    #[inline]
    pub fn memory_bus(&mut self) -> MemoryBus<'_> {
        MemoryBus {
            mem: &mut self.physical_memory,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
        }
    }

    /// Creates and initializes a complete Amiga 500 machine, constructing all
    /// chips, coprocessors, and peripheral devices in a flat structure.
    pub fn new(config: A500Config) -> Self {
        let mut cpu = Cpu::new();
        let mut physical_memory = PhysicalMemory::from_config(config.clone());
        cpu.reset(&mut physical_memory);
        let agnus = agnus::Agnus::new(config.agnus_model());
        let denise = denise::Denise::new(config.denise_model());
        let paula = paula::Paula::new(config.video_standard());
        let cia_a = cia::Cia::new();
        let cia_b = cia::Cia::new();
        let rtc = rtc::RtcMsm6242b::new(config.rtc());

        let floppy = floppy::FloppyController::new();
        let keyboard = keyboard::Keyboard::new(config.video_standard().color_clock_hz());
        let game_ports = game_ports::GamePorts::new();

        let mut machine = Self {
            keyboard_reset_asserted: false,
            config,
            cpu,
            physical_memory,
            cck: 0,
            rtc,
            agnus,
            denise,
            paula,
            cia_a,
            cia_b,
            floppy,
            keyboard,
            game_ports,
        };
        machine.route_signal_pins_end_cck();
        machine.deliver_cpu_interrupt_pins();
        machine
    }

    /// Performs cold reset: zeroes RAM, resets all chips and devices to power-on defaults
    pub fn reset(&mut self) {
        self.keyboard_reset_asserted = false;
        self.physical_memory.reset();
        self.cck = 0;
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.floppy.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.cpu.reset(&mut self.physical_memory);
        self.route_signal_pins_end_cck();
        self.deliver_cpu_interrupt_pins();
    }

    /// Performs warm reset: preserves RAM, re-engages overlay, restarts execution
    pub fn reset_warm(&mut self) {
        self.keyboard_reset_asserted = false;
        self.physical_memory.reset_warm();
        self.cck = 0;
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.floppy.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.cpu.reset_warm(&mut self.physical_memory);
        self.route_signal_pins_end_cck();
        self.deliver_cpu_interrupt_pins();
    }

    /// Resets all external devices (custom chips, CIAs, peripherals, overlay)
    /// without modifying RAM or CPU registers/PC. Invoked by M68000 `RESET` instruction.
    fn reset_external_devices(&mut self) {
        self.keyboard_reset_asserted = false;
        self.physical_memory.map_kickstart_to_low_memory();
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.floppy.reset();
        self.game_ports.reset();
        self.route_signal_pins_end_cck();
        self.deliver_cpu_interrupt_pins();
    }

    /// Advances all peer custom chips, coprocessors, and peripheral subsystems by exactly 1 Color Clock (~280 ns),
    /// dispatching matured actions, advancing RTC, and arbitrating interrupts.
    pub fn step_subsystems_cck(&mut self) {
        // Start of subsystem CCK: sample inputs before receiver work, even while waiting.
        self.sample_signal_pins_start_cck();
        // 1. Advance Agnus (steps copper, blitter, dma, raster beam counters, and mutation pipeline)
        self.agnus.step_cck_ram(&mut self.physical_memory.chip_ram);
        if let Some(address) = self.agnus.pending_dsk_dma_slot.take() {
            if let Some(word) = self.paula.take_disk_dma_word() {
                self.physical_memory.write_chip_word(address, word);
                self.agnus.complete_disk_read();
            }
        }
        self.physical_memory.chip_ram_blocked = self.agnus.chip_ram_blocked;

        // 2. Step Denise using its delivered RGA input and local receiver state.
        self.denise.step_cck();

        // 3. Step Paula (steps audio, serial_port, and mutation pipeline)
        self.paula.step_cck();
        // Common register issue boundary: every receiver has completed this CCK.
        if let Some((reg, val)) = self.agnus.pending_copper_write {
            if self.memory_bus().write_custom_word(reg, val) {
                self.agnus.poll_copper_write();
            }
        }
        if let Some((plane, word)) = self.agnus.pending_bpl_dma {
            if self.denise.set_bpldat(plane as usize, word) {
                self.agnus.poll_bpl_dma();
            }
        }
        self.deliver_dma_register();

        // 4. Step drive mechanics using captured connector control levels.
        self.floppy.step_cck();

        // Keyboard and CIA communicate only through captured KCLK/KDAT levels.
        self.keyboard.step_cck();

        // 5. Step CIAs
        self.cia_a.step_cck();
        self.cia_b.step_cck();

        // 6. Step Real-Time Clock
        self.rtc.step_cck(1);

        // 7. Route same-CCK outputs and capture pins for the next CCK input phase.
        self.route_signal_pins_end_cck();

        // 8. Central interrupt priority line (IPL 1-6) arbitration
        self.deliver_cpu_interrupt_pins();
    }

    /// Advances the entire machine by exactly 1 Color Clock (~280 ns).
    /// Subsystems receive required peer handles via method call parameters.
    /// Returns `true` if the M68000 CPU completed an instruction on this Color Clock.
    pub fn step_cck(&mut self) -> bool {
        // Retain the previous capture before subsystem routing samples the source again.
        let keyboard_reset_asserted = self.keyboard_reset_asserted;
        // 1. Advance master monotonic Color Clock counter
        self.cck = self.cck.wrapping_add(1);

        // 2. Advance non-CPU subsystems
        self.step_subsystems_cck();

        // 3. Hardware keyboard reset line (Ctrl-Amiga-Amiga)
        if keyboard_reset_asserted {
            self.keyboard.pins.reset_line_asserted = false;
            self.reset_warm();
            return false;
        }

        // 4. Sample the CPU output before stepping to detect its assertion edge.
        let reset_was_asserted = self.cpu.state.is_reset_line_asserted();

        // 5. Step CPU Color Clock phase with bus reference
        let mut bus = MemoryBus {
            mem: &mut self.physical_memory,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
        };
        let completed = self.cpu.step_cck(&mut bus);
        self.deliver_cpu_reset_pin(reset_was_asserted);
        completed
    }

    /// Advances the machine by a given number of Color Clocks
    pub fn step_cycles(&mut self, cck_count: u64) {
        for _ in 0..cck_count {
            self.step_cck();
        }
    }

    /// Executes Color Clocks until the current M68000 CPU instruction completes
    pub fn step_instruction(&mut self) {
        loop {
            let completed = self.step_cck();
            if completed || self.cpu.state.is_halted() || self.cpu.state.is_stopped() {
                break;
            }
        }
    }

    /// Executes Color Clocks until the current horizontal scanline completes (beam advances to next line)
    pub fn step_line(&mut self) {
        let current_vpos = self.agnus.vpos;
        while self.agnus.vpos == current_vpos {
            self.step_cck();
        }
    }

    /// Executes Color Clocks until a full vertical video frame completes (VBlank transition)
    pub fn step_frame(&mut self) {
        if self.agnus.vpos == 0 {
            while self.agnus.vpos == 0 {
                self.step_cck();
            }
        }
        while self.agnus.vpos != 0 {
            self.step_cck();
        }
    }

    /// Test & Debugger helper: Sets CPU PC to `target_pc` and primes prefetch pipeline
    #[inline]
    pub fn set_pc_and_prime_prefetch(&mut self, target_pc: u32) {
        let mut bus = MemoryBus {
            mem: &mut self.physical_memory,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
        };
        self.cpu.set_pc_and_prime_prefetch(target_pc, &mut bus);
    }

    /// Captures a pure machine snapshot without host I/O or preview encoding
    pub fn save_state(&self) -> A500State {
        A500State {
            keyboard_reset_asserted: self.keyboard_reset_asserted,
            cck: self.cck,
            config: self.config.clone(),
            cpu: self.cpu.state.clone(),
            physical_memory: self.physical_memory.clone(),
            rtc: self.rtc.clone(),
            agnus: self.agnus.clone(),
            denise: self.denise.clone(),
            paula: self.paula.clone(),
            cia_a: self.cia_a.clone(),
            cia_b: self.cia_b.clone(),
            floppy: self.floppy.clone(),
            keyboard: self.keyboard.clone(),
            game_ports: self.game_ports.clone(),
        }
    }

    /// Restores a complete machine state snapshot, adopting configuration and memory
    pub fn load_state(&mut self, state: &A500State) {
        // 2. Restore Machine Configuration from Save State
        self.config = state.config.clone();

        // 3. Restore Physical Memory (unconditionally restoring RAM and Kickstart ROM)
        self.physical_memory = state.physical_memory.clone();

        // 4. Restore Master Monotonic Color Clock counter
        self.cck = state.cck;

        // 5. Restore CPU state and re-hydrate static micro-step pointers
        self.cpu.restore_state(state.cpu.clone());

        // 6. Restore Custom Chips & Peripherals
        self.rtc = state.rtc.clone();
        self.agnus = state.agnus.clone();
        self.denise = state.denise.clone();
        self.paula = state.paula.clone();
        self.cia_a = state.cia_a.clone();
        self.cia_b = state.cia_b.clone();
        self.floppy = state.floppy.clone();
        self.keyboard = state.keyboard.clone();
        self.game_ports = state.game_ports.clone();
        self.keyboard_reset_asserted = state.keyboard_reset_asserted;

        // Signal levels and unconsumed output changes are serialized state.
        // Routing here would consume an event before its next modeled phase.
    }
}
