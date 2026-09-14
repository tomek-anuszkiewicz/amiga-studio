//! Amiga 500 Main Machine Loop & Subsystem Coordination
//!
//! Tier 0 top-level machine chassis that owns and coordinates all peer subsystems,
//! custom chips, coprocessors, and peripheral devices in a flat, decoupled structure.

pub use agnus::{self, blitter, copper, dma};
pub use cia;
pub use config;
pub use denise::{self, frame_builder, sprites};
pub use floppy;
pub use game_ports;
pub use joystick;
pub use keyboard;
pub use m68000;
pub use memory_bus;
pub use memory_bus::MemoryBus;
pub use mouse;
pub use parallel_port;
pub use paula::{self, audio, serial_port};
pub use physical_memory;
pub use physical_memory::{AddressBus, BusResult, PhysicalMemory};
pub use rtc;

pub mod profile;
pub use profile::*;
pub mod save_state;
pub use save_state::*;

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
        cpu.reset_cold(&mut physical_memory);
        let agnus = agnus::Agnus::new(config.agnus_model());
        let denise = denise::Denise::new(config.denise_model());
        let paula = paula::Paula::new();
        let cia_a = cia::Cia::new(cia::CiaId::A);
        let cia_b = cia::Cia::new(cia::CiaId::B);
        let rtc = rtc::RtcMsm6242b::new(config.rtc());

        let floppy = floppy::FloppyController::new();
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
            floppy,
            keyboard,
            game_ports,
            parallel_port,
        };
        machine.poll_peripheral_pins();
        machine.cpu.state.ipl = machine.resolve_ipl();
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
        self.floppy.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.parallel_port.reset();
        self.cpu.reset_cold(&mut self.physical_memory);
        self.poll_peripheral_pins();
        self.cpu.state.ipl = self.resolve_ipl();
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
        self.floppy.reset();
        self.keyboard.reset();
        self.game_ports.reset();
        self.parallel_port.reset();
        self.cpu.reset_warm(&mut self.physical_memory);
        self.poll_peripheral_pins();
        self.cpu.state.ipl = self.resolve_ipl();
    }

    /// Resets all external devices (custom chips, CIAs, peripherals, overlay)
    /// without modifying RAM or CPU registers/PC. Invoked by M68000 `RESET` instruction.
    pub fn reset_external_devices(&mut self) {
        self.physical_memory.map_kickstart_to_low_memory();
        if !self.physical_memory.is_kickstart_loaded() {
            self.physical_memory.map_chip_ram_to_low_memory();
        }
        self.agnus.reset();
        self.denise.reset();
        self.paula.reset();
        self.cia_a.reset();
        self.cia_b.reset();
        self.floppy.reset();
        self.game_ports.reset();
        self.parallel_port.reset();
        self.poll_peripheral_pins();
        self.cpu.state.ipl = self.resolve_ipl();
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
        // CIA-A (Level 2) and CIA-B (Level 6) lines pass through Paula INTENA (bits 3 and 13 + master bit 14)
        let cia_a_ipl = if self.cia_a.irq_pending()
            && (self.paula.intena == 0 || (self.paula.intena & 0x4008) == 0x4008)
        {
            2
        } else {
            0
        };
        let cia_b_ipl = if self.cia_b.irq_pending()
            && (self.paula.intena == 0 || (self.paula.intena & 0x6000) == 0x6000)
        {
            6
        } else {
            0
        };

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

    /// Polls peripheral sensing lines into CIA input pins and custom chip port latches
    pub fn poll_peripheral_pins(&mut self) {
        // 1. Floppy disk sensing lines -> CIA-A Port A bits 2..5 (mask 0x3C)
        let floppy_inputs = self.floppy.sample_ciaa_port_a_inputs();
        self.cia_a.set_input_pins_a(floppy_inputs, 0x3C);

        // 2. Game ports fire buttons -> CIA-A Port A bits 6..7 (mask 0xC0, active low)
        let mut fire_pins = 0xC0;
        if self.game_ports.fire1_port1() {
            fire_pins &= !0x40; // Bit 6 = /FIR0 (Port 1 left mouse button)
        }
        if self.game_ports.fire1_port2() {
            fire_pins &= !0x80; // Bit 7 = /FIR1 (Port 2 joystick fire 1)
        }
        self.cia_a.set_input_pins_a(fire_pins, 0xC0);

        // 3. Mouse/joystick quadrature counters -> Denise JOY0DAT & JOY1DAT
        self.denise.joy0dat = self.game_ports.joy0dat();
        self.denise.joy1dat = self.game_ports.joy1dat();

        // 4. Analog potentiometer coordinates -> Paula POT0DAT, POT1DAT & POTGOR
        self.paula.pot0dat = self.game_ports.pot0dat();
        self.paula.pot1dat = self.game_ports.pot1dat();
        self.paula.potgor = self.game_ports.potgor(self.paula.potgo);
    }

    /// Advances all peer custom chips, coprocessors, and peripheral subsystems by exactly 1 Color Clock (~280 ns),
    /// dispatching matured actions, advancing RTC, and arbitrating interrupts.
    pub fn step_subsystems_cck(&mut self) {
        // 1. Advance Agnus (steps copper, blitter, dma, raster beam counters, and mutation pipeline)
        let agnus_due = self.agnus.step_cck_ram(&mut self.physical_memory.chip_ram);
        for item in agnus_due.iter().flatten() {
            self.dispatch_agnus_action(item.0, item.1);
        }
        if let Some((reg, val)) = self.agnus.poll_copper_write() {
            self.dispatch_custom_write(reg, val);
        }
        self.physical_memory.chip_ram_blocked = self.agnus.chip_ram_blocked;

        // Cross-Chip Signal: Blitter completion (_BLITINT) -> Paula INTREQ bit 6 (mask 0x0040)
        if self.agnus.poll_blitter_irq() {
            self.paula.set_interrupt_request(0x0040);
        }

        // Cross-Chip Signal: Vertical blanking interval -> Paula INTREQ bit 5 (mask 0x0020) & CIA-A TOD tick
        if self.agnus.poll_vblank_irq() {
            self.paula.set_interrupt_request(0x0020);
            self.cia_a.tick_tod();
        }

        // 2. Step Denise (steps sprites, frame_builder, video serializer, and mutation pipeline)
        let beam = self.agnus.beam();
        if beam.hpos == 0 {
            self.cia_b.tick_tod(); // CIA-B TOD tracks horizontal scanline sync
        }
        let denise_due = self.denise.step_cck(beam);
        for item in denise_due.iter().flatten() {
            self.dispatch_denise_action(item.0, item.1);
        }

        // 3. Step Paula (steps audio, serial_port, and mutation pipeline)
        let paula_due = self.paula.step_cck();
        for item in paula_due.iter().flatten() {
            self.dispatch_paula_action(item.0, item.1);
        }
        self.floppy.step_cck();

        // Cross-Chip Signal: Disk block DMA finished -> Paula INTREQ bit 1 (mask 0x0002)
        if self.floppy.poll_dskblk_irq() {
            self.paula.set_interrupt_request(0x0002);
        }

        // Cross-Chip Signal: Audio channel buffer loop (AUDxDSR) -> Agnus audpt reload & Paula interrupt
        for ch in 0..4 {
            if self.paula.poll_audio_restart(ch) {
                self.agnus.reload_audio_ptr(ch);
                self.paula.set_interrupt_request(1u16 << (7 + ch));
            }
        }

        // 4. Step Keyboard serial transmission to CIA-A
        let kdat_handshake = self.cia_a.is_sdr_output();
        if let Some(scancode) = self.keyboard.step(kdat_handshake) {
            self.cia_a.shift_in_sdr(scancode);
        }

        // 5. Step CIAs and dispatch E-Clock mutations
        let cia_a_due = self.cia_a.step_cck();
        for item in cia_a_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::A, item.0, item.1);
        }
        let cia_b_due = self.cia_b.step_cck();
        for item in cia_b_due.iter().flatten() {
            self.dispatch_cia_action(cia::CiaId::B, item.0, item.1);
        }

        // Cross-Chip Signal: CIA-A /IRQ pin -> Paula INTREQ bit 3 (PORTS, mask 0x0008)
        if self.cia_a.irq_pending() {
            self.paula.set_interrupt_request(0x0008);
        }

        // Cross-Chip Signal: CIA-B /IRQ pin -> Paula INTREQ bit 13 (EXTER, mask 0x2000)
        if self.cia_b.irq_pending() {
            self.paula.set_interrupt_request(0x2000);
        }

        // 6. Step Real-Time Clock
        self.rtc.step_cck(1);

        // 6. Cross-Chip Cascades (Physical Pins)
        if let Some(chip_ram_engaged) = self.cia_a.ovl_transition() {
            if chip_ram_engaged {
                self.physical_memory.map_chip_ram_to_low_memory();
            } else {
                self.physical_memory.map_kickstart_to_low_memory();
            }
        }
        self.poll_peripheral_pins();

        // 7. Central interrupt priority line (IPL 1-6) arbitration
        let ipl = self.resolve_ipl();
        self.cpu.state.ipl = ipl;
    }

    /// Advances the entire machine by exactly 1 Color Clock (~280 ns).
    /// Subsystems receive required peer handles via method call parameters.
    /// Returns `true` if the M68000 CPU completed an instruction on this Color Clock.
    pub fn step_cck(&mut self) -> bool {
        // 1. Advance master monotonic Color Clock counter
        self.cck = self.cck.wrapping_add(1);

        // 2. Advance non-CPU subsystems
        self.step_subsystems_cck();

        // 3. Hardware keyboard reset line (Ctrl-Amiga-Amiga)
        if self.keyboard.reset_line_asserted {
            self.keyboard.reset_line_asserted = false;
            self.reset_warm();
            return false;
        }

        // 4. M68000 external RESET instruction pulse
        if self.cpu.state.reset_line_asserted {
            self.cpu.state.reset_line_asserted = false;
            self.reset_external_devices();
        }

        // 5. Step CPU Color Clock phase with bus reference
        let mut bus = MemoryBus {
            mem: &mut self.physical_memory,
            agnus: &mut self.agnus,
            denise: &mut self.denise,
            paula: &mut self.paula,
            cia_a: &mut self.cia_a,
            cia_b: &mut self.cia_b,
            rtc: &mut self.rtc,
            floppy: &mut self.floppy,
        };
        self.cpu.step_cck(&mut bus)
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
            if completed || self.cpu.state.halted || self.cpu.state.stopped {
                break;
            }
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

    /// Executes Color Clocks until a full vertical video frame completes (VBlank transition),
    /// measuring wall-clock duration and profiling subsystem execution.
    pub fn step_frame_profiled(&mut self, stats: &mut SubsystemProfileStats) {
        let frame_start = std::time::Instant::now();
        let cck_start = self.cck;

        let mut cpu_duration = std::time::Duration::ZERO;
        let mut agnus_duration = std::time::Duration::ZERO;
        let mut denise_duration = std::time::Duration::ZERO;
        let mut paula_duration = std::time::Duration::ZERO;
        let mut cia_duration = std::time::Duration::ZERO;

        // Sample profiling on 16 representative scanlines across the frame
        // to achieve sub-microsecond precision with < 0.1% timer overhead
        let mut in_vpos_0 = self.agnus.vpos == 0;

        loop {
            let sample_probe = (self.agnus.vpos & 0x0F) == 0;
            if sample_probe {
                self.cck = self.cck.wrapping_add(1);

                // 1. Agnus
                let t_agnus = std::time::Instant::now();
                let agnus_due = self.agnus.step_cck_ram(&mut self.physical_memory.chip_ram);
                for item in agnus_due.iter().flatten() {
                    self.dispatch_agnus_action(item.0, item.1);
                }
                if let Some((reg, val)) = self.agnus.poll_copper_write() {
                    self.dispatch_custom_write(reg, val);
                }
                self.physical_memory.chip_ram_blocked = self.agnus.chip_ram_blocked;
                if self.agnus.poll_blitter_irq() {
                    self.paula.set_interrupt_request(0x0040);
                }
                if self.agnus.poll_vblank_irq() {
                    self.paula.set_interrupt_request(0x0020);
                    self.cia_a.tick_tod();
                }
                agnus_duration += t_agnus.elapsed();

                // 2. Denise
                let t_denise = std::time::Instant::now();
                let beam = self.agnus.beam();
                if beam.hpos == 0 {
                    self.cia_b.tick_tod();
                }
                let denise_due = self.denise.step_cck(beam);
                for item in denise_due.iter().flatten() {
                    self.dispatch_denise_action(item.0, item.1);
                }
                denise_duration += t_denise.elapsed();

                // 3. Paula & Floppy
                let t_paula = std::time::Instant::now();
                let paula_due = self.paula.step_cck();
                for item in paula_due.iter().flatten() {
                    self.dispatch_paula_action(item.0, item.1);
                }
                self.floppy.step_cck();
                if self.floppy.poll_dskblk_irq() {
                    self.paula.set_interrupt_request(0x0002);
                }
                for ch in 0..4 {
                    if self.paula.poll_audio_restart(ch) {
                        self.agnus.reload_audio_ptr(ch);
                        self.paula.set_interrupt_request(1u16 << (7 + ch));
                    }
                }
                paula_duration += t_paula.elapsed();

                // 4. CIAs, Keyboard, RTC & Cascades
                let t_cia = std::time::Instant::now();
                let kdat_handshake = self.cia_a.is_sdr_output();
                if let Some(scancode) = self.keyboard.step(kdat_handshake) {
                    self.cia_a.shift_in_sdr(scancode);
                }
                let cia_a_due = self.cia_a.step_cck();
                for item in cia_a_due.iter().flatten() {
                    self.dispatch_cia_action(cia::CiaId::A, item.0, item.1);
                }
                let cia_b_due = self.cia_b.step_cck();
                for item in cia_b_due.iter().flatten() {
                    self.dispatch_cia_action(cia::CiaId::B, item.0, item.1);
                }
                if self.cia_a.irq_pending() {
                    self.paula.set_interrupt_request(0x0008);
                }
                if self.cia_b.irq_pending() {
                    self.paula.set_interrupt_request(0x2000);
                }
                self.rtc.step_cck(1);
                if let Some(chip_ram_engaged) = self.cia_a.ovl_transition() {
                    if chip_ram_engaged {
                        self.physical_memory.map_chip_ram_to_low_memory();
                    } else {
                        self.physical_memory.map_kickstart_to_low_memory();
                    }
                }
                self.poll_peripheral_pins();
                let ipl = self.resolve_ipl();
                self.cpu.state.ipl = ipl;
                cia_duration += t_cia.elapsed();

                if self.keyboard.reset_line_asserted {
                    self.keyboard.reset_line_asserted = false;
                    self.reset_warm();
                } else if self.cpu.state.reset_line_asserted {
                    self.cpu.state.reset_line_asserted = false;
                    self.reset_external_devices();
                }

                // 5. M68000 CPU & MemoryBus
                let t_cpu = std::time::Instant::now();
                let mut bus = MemoryBus {
                    mem: &mut self.physical_memory,
                    agnus: &mut self.agnus,
                    denise: &mut self.denise,
                    paula: &mut self.paula,
                    cia_a: &mut self.cia_a,
                    cia_b: &mut self.cia_b,
                    rtc: &mut self.rtc,
                    floppy: &mut self.floppy,
                };
                self.cpu.step_cck(&mut bus);
                cpu_duration += t_cpu.elapsed();
            } else {
                self.step_cck();
            }

            if in_vpos_0 {
                if self.agnus.vpos != 0 {
                    in_vpos_0 = false;
                }
            } else if self.agnus.vpos == 0 {
                break;
            }
        }

        let total_frame_duration = frame_start.elapsed();
        let cck_delta = self.cck.wrapping_sub(cck_start);

        // Scale sampled probe ratios to match total frame duration
        let sampled_total =
            cpu_duration + agnus_duration + denise_duration + paula_duration + cia_duration;
        if sampled_total.as_nanos() > 0 {
            let total_secs = total_frame_duration.as_secs_f64();
            let sampled_secs = sampled_total.as_secs_f64();
            let scale = total_secs / sampled_secs;
            stats.cpu_duration += cpu_duration.mul_f64(scale);
            stats.agnus_duration += agnus_duration.mul_f64(scale);
            stats.denise_duration += denise_duration.mul_f64(scale);
            stats.paula_duration += paula_duration.mul_f64(scale);
            stats.cia_duration += cia_duration.mul_f64(scale);
        } else {
            stats.cpu_duration += total_frame_duration / 5;
            stats.agnus_duration += total_frame_duration / 5;
            stats.denise_duration += total_frame_duration / 5;
            stats.paula_duration += total_frame_duration / 5;
            stats.cia_duration += total_frame_duration / 5;
        }

        stats.total_duration += total_frame_duration;
        stats.frames_executed += 1;
        stats.cck_executed += cck_delta;
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
            floppy: &mut self.floppy,
        };
        self.cpu.set_pc_and_prime_prefetch(target_pc, &mut bus);
    }

    /// Captures a complete machine state snapshot in referenced Kickstart ROM mode
    pub fn save_state(&self) -> A500State {
        let kickstart_crc32 = compute_crc32(&self.physical_memory.kickstart_rom);
        let header = SaveStateHeader {
            magic: SAVE_STATE_MAGIC,
            version: SAVE_STATE_VERSION,
            timestamp: 0,
            video_standard: self.config.video_standard(),
            chip_ram_size: self.physical_memory.chip_ram.len(),
            slow_ram_size: self
                .physical_memory
                .slow_ram
                .as_ref()
                .map_or(0, |r| r.len()),
            fast_ram_size: self
                .physical_memory
                .fast_ram
                .as_ref()
                .map_or(0, |r| r.len()),
            kickstart_crc32,
            is_self_contained: false,
        };

        let mut mem_clone = self.physical_memory.clone();
        // In referenced mode, do not duplicate Kickstart ROM in the snapshot
        mem_clone.kickstart_rom = Vec::new();

        A500State {
            header,
            cck: self.cck,
            config: self.config.clone(),
            cpu: self.cpu.state.clone(),
            physical_memory: mem_clone,
            rtc: self.rtc.clone(),
            agnus: self.agnus.clone(),
            denise: self.denise.clone(),
            paula: self.paula.clone(),
            cia_a: self.cia_a.clone(),
            cia_b: self.cia_b.clone(),
            floppy: self.floppy.clone(),
            keyboard: self.keyboard.clone(),
            game_ports: self.game_ports.clone(),
            parallel_port: self.parallel_port.clone(),
        }
    }

    /// Captures a self-contained state snapshot with embedded Kickstart ROM
    pub fn save_state_self_contained(&self) -> A500State {
        let mut state = self.save_state();
        state.header.is_self_contained = true;
        state.physical_memory.kickstart_rom = self.physical_memory.kickstart_rom.clone();
        state
    }

    /// Restores a complete machine state snapshot with strict compatibility guards
    pub fn load_state(&mut self, state: &A500State) -> Result<(), SaveStateError> {
        // 1. Verify Magic header
        if state.header.magic != SAVE_STATE_MAGIC {
            return Err(SaveStateError::InvalidMagic);
        }

        // 2. Verify Schema Version
        if state.header.version != SAVE_STATE_VERSION {
            return Err(SaveStateError::IncompatibleVersion {
                found: state.header.version,
                supported: SAVE_STATE_VERSION,
            });
        }

        // 3. Verify Chip RAM configuration compatibility
        let current_chip_size = self.physical_memory.chip_ram.len();
        if state.header.chip_ram_size != current_chip_size {
            return Err(SaveStateError::MemorySizeMismatch {
                expected_chip: state.header.chip_ram_size,
                actual_chip: current_chip_size,
            });
        }

        // 4. Verify Kickstart ROM compatibility
        if state.header.is_self_contained {
            self.physical_memory.kickstart_rom = state.physical_memory.kickstart_rom.clone();
        } else if self.physical_memory.is_kickstart_loaded() {
            let active_crc = compute_crc32(&self.physical_memory.kickstart_rom);
            if state.header.kickstart_crc32 != 0 && active_crc != state.header.kickstart_crc32 {
                return Err(SaveStateError::KickstartMismatch {
                    expected_crc: state.header.kickstart_crc32,
                    actual_crc: active_crc,
                });
            }
        }

        // 5. Restore Physical Memory (preserving active ROM in referenced mode)
        let active_rom = self.physical_memory.kickstart_rom.clone();
        self.physical_memory = state.physical_memory.clone();
        if !state.header.is_self_contained {
            self.physical_memory.kickstart_rom = active_rom;
        }

        // 6. Restore Master Monotonic Color Clock counter
        self.cck = state.cck;

        // 7. Restore CPU state and re-hydrate static micro-step pointers
        self.cpu.state = state.cpu.clone();
        self.cpu.rehydrate_micro_steps();

        // 8. Restore Custom Chips & Peripherals
        self.rtc = state.rtc.clone();
        self.agnus = state.agnus.clone();
        self.denise = state.denise.clone();
        self.paula = state.paula.clone();
        self.cia_a = state.cia_a.clone();
        self.cia_b = state.cia_b.clone();
        self.floppy = state.floppy.clone();
        self.keyboard = state.keyboard.clone();
        self.game_ports = state.game_ports.clone();
        self.parallel_port = state.parallel_port.clone();

        // 9. Re-poll peripheral pins to establish consistent signal line levels
        self.poll_peripheral_pins();

        Ok(())
    }

    /// Saves the machine state snapshot to a file (compressed if requested or matching extension)
    pub fn save_state_to_file(
        &self,
        path: impl AsRef<std::path::Path>,
        self_contained: bool,
    ) -> Result<(), SaveStateError> {
        let state = if self_contained {
            self.save_state_self_contained()
        } else {
            self.save_state()
        };
        let path = path.as_ref();
        let is_gz = path
            .extension()
            .map_or(false, |ext| ext == "gz" || ext == "a500z");
        state.save_to_file(path, is_gz)
    }

    /// Restores the machine state from a file, automatically detecting format/compression
    pub fn load_state_from_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), SaveStateError> {
        let state = A500State::load_from_file(path)?;
        self.load_state(&state)
    }
}
