//! Amiga 500 Main Machine Loop & Subsystem Coordination
//!
//! Tier 0 top-level machine chassis that owns and coordinates all peer subsystems,
//! custom chips, coprocessors, and peripheral devices in a flat, decoupled structure.

pub use agnus;
pub use audio;
pub use blitter;
pub use cia;
pub use config;
pub use copper;
pub use denise;
pub use dma;
pub use floppy;
pub use frame_builder;
pub use game_ports;
pub use joystick;
pub use keyboard;
pub use m68000;
pub use memory_bus;
pub use mouse;
pub use parallel_port;
pub use paula;
pub use serial_port;
pub use sprites;

use config::A500Config;
use m68000::Cpu;
use memory_bus::MemoryBus;

/// Top-level Amiga 500 machine struct orchestrating all subsystems
#[derive(Debug, Clone)]
pub struct A500Machine {
    /// Active hardware configuration
    pub config: A500Config,
    /// Motorola 68000 cycle-exact CPU core
    pub cpu: Cpu,
    /// 24-bit physical address space and memory banks
    pub memory_bus: MemoryBus,
    /// Monotonic master 64-bit Color Clock counter (CCK)
    pub cck: u64,

    // --- Custom Chipsets (Układy) ---
    /// Agnus (beam counters, chip RAM bus arbitration)
    pub agnus: agnus::Agnus,
    /// Denise (video control, palette, collision registers)
    pub denise: denise::Denise,
    /// Paula (interrupt request / enable arbitration)
    pub paula: paula::Paula,
    /// CIA-A (timers, TOD, port A/B, keyboard interface, overlay)
    pub cia_a: cia::Cia,
    /// CIA-B (timers, TOD, port A/B, parallel handshakes)
    pub cia_b: cia::Cia,

    // --- Coprocessors & Display Engines (Wyspecjalizowane Części) ---
    /// Agnus Copper coprocessor
    pub copper: copper::Copper,
    /// Agnus 4-channel DMA Blitter
    pub blitter: blitter::Blitter,
    /// Agnus DMA slot arbiter & scheduler
    pub dma: dma::DmaScheduler,
    /// Denise 8 hardware sprite engines
    pub sprites: sprites::Sprites,
    /// Denise raster scanline pixel compositor & frame buffer
    pub frame_builder: frame_builder::FrameBuilder,
    /// Paula 4-channel 8-bit DMA audio engine
    pub audio: audio::Audio,

    // --- Peripherals & Controller Devices (Urządzenia) ---
    /// Paula 3.5" DD floppy disk drive controller & mechanics
    pub floppy: floppy::FloppyController,
    /// Paula RS-232 serial UART transceiver
    pub serial_port: serial_port::SerialPort,
    /// MOS 6500/1 keyboard microcontroller with reset logic
    pub keyboard: keyboard::Keyboard,
    /// Amiga dual controller game ports (Port 1 Mouse / Port 2 Joystick)
    pub game_ports: game_ports::GamePorts,
    /// Centronics 8-bit parallel printer port interface
    pub parallel_port: parallel_port::ParallelPort,
}

impl A500Machine {
    /// Creates and initializes a complete Amiga 500 machine, constructing all
    /// chips, coprocessors, and peripheral devices in a flat structure.
    pub fn new(config: A500Config) -> Self {
        let mut cpu = Cpu::new();
        let mut memory_bus = MemoryBus::from_config(config.clone());
        if !memory_bus.is_kickstart_loaded() {
            memory_bus.map_chip_ram_to_low_memory();
        }
        cpu.reset(&mut memory_bus);
        let agnus = agnus::Agnus::new(config.agnus_model());
        let denise = denise::Denise::new(config.denise_model());
        let paula = paula::Paula::new();
        let cia_a = cia::Cia::new(cia::CiaId::A);
        let cia_b = cia::Cia::new(cia::CiaId::B);

        let copper = copper::Copper::new();
        let blitter = blitter::Blitter::new();
        let dma = dma::DmaScheduler::new();
        let sprites = sprites::Sprites::new();
        let frame_builder = frame_builder::FrameBuilder::new();
        let audio = audio::Audio::new();

        let floppy = floppy::FloppyController::new();
        let serial_port = serial_port::SerialPort::new();
        let keyboard = keyboard::Keyboard::new();
        let game_ports = game_ports::GamePorts::new();
        let parallel_port = parallel_port::ParallelPort::new();

        Self {
            config,
            cpu,
            memory_bus,
            cck: 0,
            agnus,
            denise,
            paula,
            cia_a,
            cia_b,
            copper,
            blitter,
            dma,
            sprites,
            frame_builder,
            audio,
            floppy,
            serial_port,
            keyboard,
            game_ports,
            parallel_port,
        }
    }

    /// Performs cold reset: zeroes RAM, resets all chips and devices to power-on defaults
    pub fn reset_cold(&mut self) {
        self.memory_bus.reset_cold();
        if !self.memory_bus.is_kickstart_loaded() {
            self.memory_bus.map_chip_ram_to_low_memory();
        }
        self.cck = 0;
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.copper.reset();
        self.blitter.reset();
        self.dma.reset();
        self.sprites.reset();
        self.frame_builder.reset();
        self.audio.reset();
        self.floppy.reset();
        self.serial_port.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.parallel_port.reset();
        self.cpu.reset(&mut self.memory_bus);
    }

    /// Performs warm reset: preserves RAM, re-engages overlay, restarts execution
    pub fn reset_warm(&mut self) {
        self.memory_bus.reset_warm();
        if !self.memory_bus.is_kickstart_loaded() {
            self.memory_bus.map_chip_ram_to_low_memory();
        }
        self.cck = 0;
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.copper.reset();
        self.blitter.reset();
        self.dma.reset();
        self.sprites.reset();
        self.frame_builder.reset();
        self.audio.reset();
        self.floppy.reset();
        self.serial_port.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.parallel_port.reset();
        self.cpu.reset(&mut self.memory_bus);
    }

    /// Applies relative mouse movement deltas to the connected mouse (Port 1 default)
    #[inline]
    pub fn apply_mouse_delta(&mut self, dx: i32, dy: i32) {
        self.game_ports.apply_mouse_delta(dx, dy);
    }

    /// Sets mouse button states on the connected mouse (Port 1 default)
    #[inline]
    pub fn set_mouse_buttons(&mut self, left: bool, right: bool, middle: bool) {
        self.game_ports.set_mouse_buttons(left, right, middle);
    }

    /// Sets joystick directional switches and fire buttons on the connected joystick (Port 2 default)
    #[inline]
    pub fn set_joystick(
        &mut self,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        fire1: bool,
        fire2: bool,
    ) {
        self.game_ports
            .set_joystick(up, down, left, right, fire1, fire2);
    }

    /// Resolves the highest pending interrupt level across Paula, CIA-A, and CIA-B
    #[inline]
    pub fn resolve_ipl(&self) -> u8 {
        let paula_ipl = self.paula.pending_interrupt_level();
        let cia_a_ipl = if self.cia_a.irq_pending() { 2 } else { 0 };
        let cia_b_ipl = if self.cia_b.irq_pending() { 6 } else { 0 };

        paula_ipl.max(cia_a_ipl).max(cia_b_ipl)
    }

    /// Advances the entire machine by exactly 1 Color Clock (~280 ns).
    /// Subsystems receive required peer handles via method call parameters.
    pub fn step_cck(&mut self) {
        // 1. Advance master monotonic Color Clock counter
        self.cck = self.cck.wrapping_add(1);

        // 2. Advance Agnus raster beam counters
        self.agnus.step_cck();

        // 3. Step Copper coprocessor
        self.copper.step_cck();

        // 4. Step Blitter engine
        self.blitter.step_cck();

        // 5. Step DMA scheduler and evaluate Chip RAM contention
        self.dma.step_cck();
        self.agnus.chip_ram_blocked = self
            .dma
            .is_chip_ram_blocked(self.agnus.hpos, self.blitter.is_busy);

        // 6. Step Denise video serializer
        self.denise.step_cck();

        // 7. Step Paula audio, floppy, and serial transceivers
        self.audio.step_cck();
        self.floppy.step_cck();
        self.serial_port.step_cck();
        self.paula.step_cck();

        // 8. Step CIAs
        self.cia_a.step_cck();
        self.cia_b.step_cck();

        // 9. Central interrupt priority line (IPL 1-6) arbitration
        let ipl = self.resolve_ipl();
        self.cpu.state.ipl = ipl;

        // 10. Step CPU Color Clock phase with bus reference
        self.cpu.step_cck(&mut self.memory_bus);
    }

    /// Advances the machine by a given number of Color Clocks
    pub fn step_cycles(&mut self, cck_count: u64) {
        for _ in 0..cck_count {
            self.step_cck();
        }
    }

    /// Executes Color Clocks until the current M68000 CPU instruction completes
    pub fn step_instruction(&mut self) {
        self.step_cck();
        while self.cpu.state.micro.micro_step != 0 {
            self.step_cck();
        }
    }

    /// Executes Color Clocks until a full vertical video frame completes (VBlank transition)
    pub fn step_frame(&mut self) {
        let initial_vpos = self.agnus.vpos;
        self.step_cck();
        while !(self.agnus.vpos == 0 && initial_vpos != 0) {
            self.step_cck();
        }
    }
}
