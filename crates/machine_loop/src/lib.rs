//! Amiga 500 Main Machine Loop & Subsystem Coordination
//!
//! Tier 0 top-level machine chassis that owns and coordinates all peer subsystems,
//! custom chips, coprocessors, and peripheral devices in a flat, decoupled structure.

pub mod bus;

pub use agnus;
pub use audio;
pub use blitter;
pub use bus::MemoryBus;
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
pub use memory_bus::{AddressBus, PhysicalMemory};
pub use mouse;
pub use parallel_port;
pub use paula;
pub use rtc;
pub use serial_port;
pub use sprites;

use config::A500Config;
use m68000::Cpu;

/// Top-level Amiga 500 machine struct orchestrating all subsystems
#[derive(Debug, Clone)]
pub struct A500Machine {
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

    // --- Coprocessors & Display Engines ---
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

    // --- Peripherals & Controller Devices ---
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
            copper: &mut self.copper,
            blitter: &mut self.blitter,
            dma: &mut self.dma,
            sprites: &mut self.sprites,
            frame_builder: &mut self.frame_builder,
            audio: &mut self.audio,
            floppy: &mut self.floppy,
        }
    }

    /// Creates and initializes a complete Amiga 500 machine, constructing all
    /// chips, coprocessors, and peripheral devices in a flat structure.
    pub fn new(config: A500Config) -> Self {
        let mut cpu = Cpu::new();
        let mut physical_memory = PhysicalMemory::from_config(config.clone());
        if !physical_memory.is_kickstart_loaded() {
            physical_memory.map_chip_ram_to_low_memory();
        }
        cpu.reset(&mut physical_memory);
        let agnus = agnus::Agnus::new(config.agnus_model());
        let denise = denise::Denise::new(config.denise_model());
        let paula = paula::Paula::new();
        let cia_a = cia::Cia::new(cia::CiaId::A);
        let cia_b = cia::Cia::new(cia::CiaId::B);
        let rtc = rtc::RtcMsm6242b::new(config.rtc());

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

        let mut machine = Self {
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
        };
        machine.poll_peripheral_pins();
        machine
    }

    /// Performs cold reset: zeroes RAM, resets all chips and devices to power-on defaults
    pub fn reset_cold(&mut self) {
        self.physical_memory.reset_cold();
        if !self.physical_memory.is_kickstart_loaded() {
            self.physical_memory.map_chip_ram_to_low_memory();
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
        self.cpu.reset(&mut self.physical_memory);
    }

    /// Performs warm reset: preserves RAM, re-engages overlay, restarts execution
    pub fn reset_warm(&mut self) {
        self.physical_memory.reset_warm();
        if !self.physical_memory.is_kickstart_loaded() {
            self.physical_memory.map_chip_ram_to_low_memory();
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
        self.cpu.reset(&mut self.physical_memory);
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

    /// Dispatches a custom register bus write to the target chip(s) with physical propagation delay.
    pub fn dispatch_custom_write(&mut self, offset: u16, val: u16) {
        self.memory_bus().dispatch_custom_write(offset, val);
    }

    /// Action method dispatch for committed Agnus registers
    pub fn dispatch_agnus_action(&mut self, reg: u16, val: u16) {
        self.memory_bus().dispatch_agnus_action(reg, val);
    }

    /// Action method dispatch for committed Paula registers
    pub fn dispatch_paula_action(&mut self, reg: u16, val: u16) {
        self.memory_bus().dispatch_paula_action(reg, val);
    }

    /// Action method dispatch for committed Denise registers
    pub fn dispatch_denise_action(&mut self, reg: u16, val: u16) {
        self.memory_bus().dispatch_denise_action(reg, val);
    }

    /// Action method dispatch for committed CIA registers
    pub fn dispatch_cia_action(&mut self, id: cia::CiaId, reg: u8, val: u8) {
        self.memory_bus().dispatch_cia_action(id, reg, val);
    }

    /// Polls peripheral sensing lines into CIA input pins
    pub fn poll_peripheral_pins(&mut self) {
        let floppy_inputs = self.floppy.sample_ciaa_port_a_inputs();
        self.cia_a.set_input_pins_a(floppy_inputs, 0x3C);
    }

    /// Advances all peer custom chips, coprocessors, and peripheral subsystems by exactly 1 Color Clock (~280 ns),
    /// dispatching matured actions, advancing RTC, and arbitrating interrupts.
    pub fn step_subsystems_cck(&mut self) {
        // 1. Advance Agnus raster beam counters and mutation pipeline
        let agnus_due = self.agnus.step_cck();
        for item in agnus_due.iter().flatten() {
            self.dispatch_agnus_action(item.0, item.1);
        }
        let beam = self.agnus.beam();

        // 2. Step Copper coprocessor with beam coordinates
        self.copper.step_cck(beam);

        // 3. Step Blitter engine
        self.blitter.step_cck();

        // 4. Step DMA scheduler and evaluate Chip RAM contention
        self.dma.step_cck();
        self.agnus.chip_ram_blocked = self
            .dma
            .is_chip_ram_blocked(beam.hpos, self.blitter.is_busy);
        self.physical_memory.chip_ram_blocked = self.agnus.chip_ram_blocked;

        // 5. Step Denise video serializer and mutation pipeline
        let denise_due = self.denise.step_cck();
        for item in denise_due.iter().flatten() {
            self.dispatch_denise_action(item.0, item.1);
        }
        self.sprites.step_cck(beam);
        self.frame_builder.step_cck(beam);

        // 6. Step Paula audio, floppy, serial transceivers, and mutation pipeline
        let paula_due = self.paula.step_cck();
        for item in paula_due.iter().flatten() {
            self.dispatch_paula_action(item.0, item.1);
        }
        self.audio.step_cck();
        self.floppy.step_cck();
        self.serial_port.step_cck();

        // 7. Step CIAs and dispatch E-Clock mutations
        let cia_a_due = self.cia_a.step_cck();
        for item in cia_a_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::A, item.0, item.1);
        }
        let cia_b_due = self.cia_b.step_cck();
        for item in cia_b_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::B, item.0, item.1);
        }

        // 8. Step Real-Time Clock
        self.rtc.step_cck(1);

        // 9. Cross-Chip Cascades (Physical Pins)
        if let Some(chip_ram_engaged) = self.cia_a.ovl_transition() {
            if chip_ram_engaged {
                self.physical_memory.map_chip_ram_to_low_memory();
            } else {
                self.physical_memory.map_kickstart_to_low_memory();
            }
        }
        self.poll_peripheral_pins();

        // 10. Central interrupt priority line (IPL 1-6) arbitration
        let ipl = self.resolve_ipl();
        self.cpu.state.ipl = ipl;
    }

    /// Advances the entire machine by exactly 1 Color Clock (~280 ns).
    /// Subsystems receive required peer handles via method call parameters.
    pub fn step_cck(&mut self) {
        // 1. Advance master monotonic Color Clock counter
        self.cck = self.cck.wrapping_add(1);

        // 2. Advance non-CPU subsystems
        self.step_subsystems_cck();

        // 3. Step CPU Color Clock phase with bus reference
        let mut bus = MemoryBus {
            mem: &mut self.physical_memory,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
            copper: &mut self.copper,
            blitter: &mut self.blitter,
            dma: &mut self.dma,
            sprites: &mut self.sprites,
            frame_builder: &mut self.frame_builder,
            audio: &mut self.audio,
            floppy: &mut self.floppy,
        };
        self.cpu.step_cck(&mut bus);
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
