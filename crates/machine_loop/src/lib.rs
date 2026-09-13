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

        let mut machine = Self {
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
        };
        machine.poll_peripheral_pins();
        machine.sync_memory_bus_registers();
        machine
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

    /// Dispatches a custom register bus write to the target chip(s) with physical propagation delay.
    /// If any write commits immediately (e.g. overflow fallback or 0 delay), triggers its action immediately.
    pub fn dispatch_custom_write(&mut self, offset: u16, val: u16) {
        let offset = offset & 0x1FE;
        match offset {
            // Shared / Broadcast: DMACON ($096) -> Agnus and Paula
            0x096 => {
                if let Some((r, v)) = self.agnus.write_register(0x096, val) {
                    self.dispatch_agnus_action(r, v);
                }
                if let Some((r, v)) = self.paula.write_register(0x096, val) {
                    self.dispatch_paula_action(r, v);
                }
            }
            // Shared / Broadcast: BPLCON0 ($100) -> Denise (1 CCK) & Agnus (4 CCK)
            0x100 => {
                if let Some((r, v)) = self.denise.write_register(0x100, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(0x100, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Shared / Broadcast: BPLCON1 ($102) -> Denise & Agnus
            0x102 => {
                if let Some((r, v)) = self.denise.write_register(0x102, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(0x102, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Shared / Broadcast: DIWSTRT ($08E) & DIWSTOP ($090) -> Denise & Agnus
            0x08E | 0x090 => {
                if let Some((r, v)) = self.denise.write_register(offset, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(offset, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Denise-specific registers (CLXCON, BPLCON2/3, BPLDAT, SPRITES, COLORS)
            0x098 | 0x104 | 0x106 | 0x110..=0x11A | 0x140..=0x17E | 0x180..=0x1BE | 0x036 => {
                if let Some((r, v)) = self.denise.write_register(offset, val) {
                    self.dispatch_denise_action(r, v);
                }
            }
            // Paula-specific registers (INTENA, INTREQ, ADKCON, UART, DSKLEN/SYNC, AUDIO)
            0x09A
            | 0x09C
            | 0x09E
            | 0x018
            | 0x01A
            | 0x024
            | 0x026
            | 0x030..=0x034
            | 0x07E
            | 0x0A0..=0x0DE => {
                if let Some((r, v)) = self.paula.write_register(offset, val) {
                    self.dispatch_paula_action(r, v);
                }
            }
            // Agnus-specific registers (Blitter, Copper, DMA pointers, modulos, DDF)
            _ => {
                if let Some((r, v)) = self.agnus.write_register(offset, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
        }
    }

    /// Action method dispatch for committed Agnus registers
    pub fn dispatch_agnus_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            0x096 => {
                // DMACON channel routing
                self.dma.write_dmacon(val);
                let dmaen = (self.agnus.dmacon & 0x0200) != 0;
                self.audio
                    .set_dma_enables((self.agnus.dmacon & 0x000F) as u8, dmaen);
                self.floppy
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0010) != 0);
                self.sprites
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0020) != 0);
                self.blitter
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0040) != 0);
                self.blitter.set_bltpri((self.agnus.dmacon & 0x0400) != 0);
                self.copper
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0080) != 0);
                self.frame_builder
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0100) != 0);
            }
            0x088 => {
                self.copper.strobe_jump1(self.agnus.cop1lc);
            }
            0x08A => {
                self.copper.strobe_jump2(self.agnus.cop2lc);
            }
            0x080 | 0x082 => {
                self.copper.set_cop1lc(self.agnus.cop1lc);
            }
            0x084 | 0x086 => {
                self.copper.set_cop2lc(self.agnus.cop2lc);
            }
            0x02E => {
                self.copper.set_copcon(val);
            }
            0x058 => {
                self.blitter.sync_pointers(
                    self.agnus.bltapt,
                    self.agnus.bltbpt,
                    self.agnus.bltcpt,
                    self.agnus.bltdpt,
                );
                self.blitter.sync_controls(
                    self.agnus.bltcon0,
                    self.agnus.bltcon1,
                    self.agnus.bltafwm,
                    self.agnus.bltalwm,
                    self.agnus.bltamod,
                    self.agnus.bltbmod,
                    self.agnus.bltcmod,
                    self.agnus.bltdmod,
                );
                self.blitter.trigger_blit(val);
            }
            0x100 => {
                self.denise.set_bplcon0(val);
            }
            0x102 => {
                self.denise.set_bplcon1(val);
            }
            0x08E | 0x090 => {
                self.denise.set_diw(self.agnus.diwstrt, self.agnus.diwstop);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed Paula registers
    pub fn dispatch_paula_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            // Audio channel 0
            0x0A4 => self.audio.set_len(0, val),
            0x0A6 => self.audio.set_per(0, val),
            0x0A8 => self.audio.set_vol(0, (val & 0x7F) as u8),
            0x0AA => self.audio.set_dat(0, val),

            // Audio channel 1
            0x0B4 => self.audio.set_len(1, val),
            0x0B6 => self.audio.set_per(1, val),
            0x0B8 => self.audio.set_vol(1, (val & 0x7F) as u8),
            0x0BA => self.audio.set_dat(1, val),

            // Audio channel 2
            0x0C4 => self.audio.set_len(2, val),
            0x0C6 => self.audio.set_per(2, val),
            0x0C8 => self.audio.set_vol(2, (val & 0x7F) as u8),
            0x0CA => self.audio.set_dat(2, val),

            // Audio channel 3
            0x0D4 => self.audio.set_len(3, val),
            0x0D6 => self.audio.set_per(3, val),
            0x0D8 => self.audio.set_vol(3, (val & 0x7F) as u8),
            0x0DA => self.audio.set_dat(3, val),

            // Floppy disk controller registers
            0x020 | 0x022 => self.floppy.set_dskpt(self.agnus.dskpt),
            0x024 => self.floppy.set_dsklen(val),
            0x07E => self.floppy.set_dsksyn(val),
            0x09E => self.floppy.set_adkcon(self.paula.adkcon),
            0x096 => {
                let dmaen = (self.agnus.dmacon & 0x0200) != 0;
                self.floppy
                    .set_dma_enabled(dmaen && (self.paula.dma_enables & 0x0010) != 0);
                self.audio
                    .set_dma_enables((self.paula.dma_enables & 0x000F) as u8, dmaen);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed Denise registers
    pub fn dispatch_denise_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            0x100 => self.denise.set_bplcon0(val),
            0x102 => self.denise.set_bplcon1(val),
            0x104 => self.denise.set_bplcon2(val),
            0x180..=0x1BE => {
                let idx = ((reg - 0x180) / 2) as usize;
                self.denise.set_color(idx, val);
            }
            0x08E | 0x090 => {
                self.denise
                    .set_diw(self.denise.diwstrt, self.denise.diwstop);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed CIA registers
    pub fn dispatch_cia_action(&mut self, id: cia::CiaId, reg: u8, val: u8) {
        if id == cia::CiaId::B && (reg & 0x0F) == 0x1 {
            // CIA-B Port B ($BFD100): drive motor latching, stepping, side select, and unit select
            self.floppy.handle_ciab_port_b_write(val);
        }
    }

    /// Polls peripheral sensing lines into CIA input pins
    pub fn poll_peripheral_pins(&mut self) {
        let floppy_inputs = self.floppy.sample_ciaa_port_a_inputs();
        self.cia_a.set_input_pins_a(floppy_inputs, 0x3C);
    }

    /// Synchronizes active chip register values into the MemoryBus read snapshot
    #[inline]
    pub fn sync_memory_bus_registers(&mut self) {
        self.memory_bus.custom_registers[0x002 >> 1] = self.agnus.read_dmaconr();
        self.memory_bus.custom_registers[0x004 >> 1] = self.agnus.vposr();
        self.memory_bus.custom_registers[0x006 >> 1] = self.agnus.vhposr();
        self.memory_bus.custom_registers[0x00A >> 1] = self.denise.joy0dat;
        self.memory_bus.custom_registers[0x00C >> 1] = self.denise.joy1dat;
        self.memory_bus.custom_registers[0x00E >> 1] = self.denise.peek_register(0x00E);
        self.memory_bus.custom_registers[0x010 >> 1] = self.paula.adkcon;
        self.memory_bus.custom_registers[0x012 >> 1] = self.paula.pot0dat;
        self.memory_bus.custom_registers[0x014 >> 1] = self.paula.pot1dat;
        self.memory_bus.custom_registers[0x016 >> 1] = self.paula.potgor;
        self.memory_bus.custom_registers[0x018 >> 1] = self.paula.serdatr;
        self.memory_bus.custom_registers[0x01A >> 1] = self.paula.dskbytr;
        self.memory_bus.custom_registers[0x01C >> 1] = self.paula.intena;
        self.memory_bus.custom_registers[0x01E >> 1] = self.paula.intreq;
        for i in 0..16 {
            self.memory_bus.cia_a_registers[i] = self.cia_a.peek_register(i as u8);
            self.memory_bus.cia_b_registers[i] = self.cia_b.peek_register(i as u8);
        }
    }

    /// Advances all peer custom chips, coprocessors, and peripheral subsystems by exactly 1 Color Clock (~280 ns),
    /// draining pending bus writes, dispatching matured actions, syncing registers, and arbitrating interrupts.
    pub fn step_subsystems_cck(&mut self) {
        // 1. Drain and dispatch pending bus writes to target custom chips and CIAs
        while let Some(event) = self.memory_bus.pop_custom_write() {
            self.dispatch_custom_write(event.offset, event.val);
        }
        while let Some(event) = self.memory_bus.pop_cia_write() {
            if event.is_cia_b {
                if let Some((r, v)) = self.cia_b.write_register(event.reg, event.val) {
                    self.dispatch_cia_action(cia::CiaId::B, r, v);
                }
            } else {
                if let Some((r, v)) = self.cia_a.write_register(event.reg, event.val) {
                    self.dispatch_cia_action(cia::CiaId::A, r, v);
                }
            }
        }

        // 2. Advance Agnus raster beam counters and mutation pipeline
        let agnus_due = self.agnus.step_cck();
        for item in agnus_due.iter().flatten() {
            self.dispatch_agnus_action(item.0, item.1);
        }
        let beam = self.agnus.beam();

        // 3. Step Copper coprocessor with beam coordinates
        self.copper.step_cck(beam);

        // 4. Step Blitter engine
        self.blitter.step_cck();

        // 5. Step DMA scheduler and evaluate Chip RAM contention
        self.dma.step_cck();
        self.agnus.chip_ram_blocked = self
            .dma
            .is_chip_ram_blocked(beam.hpos, self.blitter.is_busy);
        self.memory_bus.chip_ram_blocked = self.agnus.chip_ram_blocked;

        // 6. Step Denise video serializer and mutation pipeline
        let denise_due = self.denise.step_cck();
        for item in denise_due.iter().flatten() {
            self.dispatch_denise_action(item.0, item.1);
        }
        self.sprites.step_cck(beam);
        self.frame_builder.step_cck(beam);

        // 7. Step Paula audio, floppy, serial transceivers, and mutation pipeline
        let paula_due = self.paula.step_cck();
        for item in paula_due.iter().flatten() {
            self.dispatch_paula_action(item.0, item.1);
        }
        self.audio.step_cck();
        self.floppy.step_cck();
        self.serial_port.step_cck();

        // 8. Step CIAs and dispatch E-Clock mutations
        let cia_a_due = self.cia_a.step_cck();
        for item in cia_a_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::A, item.0, item.1);
        }
        let cia_b_due = self.cia_b.step_cck();
        for item in cia_b_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::B, item.0, item.1);
        }

        // 9. Cross-Chip Cascades (Physical Pins)
        if let Some(chip_ram_engaged) = self.cia_a.ovl_transition() {
            if chip_ram_engaged {
                self.memory_bus.map_chip_ram_to_low_memory();
            } else {
                self.memory_bus.map_kickstart_to_low_memory();
            }
        }
        self.poll_peripheral_pins();

        // 10. Sync active chip registers into the MemoryBus read snapshot
        self.sync_memory_bus_registers();

        // 11. Central interrupt priority line (IPL 1-6) arbitration
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
