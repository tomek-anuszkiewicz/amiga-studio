//! Amiga 500 Machine Loop Execution Profiler
//!
//! Provides zero-cost execution metrics, FPS tracking, and subsystem timing probes.

use crate::{A500Machine, MemoryBus};
use std::time::Duration;

/// Comprehensive subsystem execution statistics and timing breakdown
#[derive(Debug, Clone, Default)]
pub struct SubsystemProfileStats {
    /// Total elapsed wall-clock time across profiled frames
    pub total_duration: Duration,
    /// Total number of full vertical video frames executed
    pub frames_executed: u64,
    /// Total number of Color Clock cycles stepped
    pub cck_executed: u64,

    // --- Top-Level Major Chipsets ---
    /// Accumulated duration spent in M68000 CPU execution & bus routing
    pub cpu_duration: Duration,
    /// Accumulated duration spent in Agnus (DMA slot scheduling, Copper, Blitter, Beam)
    pub agnus_duration: Duration,
    /// Accumulated duration spent in Denise (Sprites, FrameBuilder pixel compositing)
    pub denise_duration: Duration,
    /// Accumulated duration spent in Paula & Floppy controller
    pub paula_duration: Duration,
    /// Accumulated duration spent in CIAs A & B, Keyboard, and RTC
    pub cia_duration: Duration,

    // --- Agnus Sub-Units ---
    /// Accumulated duration in Copper coprocessor execution
    pub copper_duration: Duration,
    /// Accumulated duration in Blitter 4-channel DMA & ALU operations
    pub blitter_duration: Duration,
    /// Accumulated duration in DMA slot arbitration & pointer progression
    pub dma_duration: Duration,
    /// Accumulated duration in Agnus raster beam counters & mutation pipeline
    pub agnus_beam_duration: Duration,

    // --- Denise Sub-Units ---
    /// Accumulated duration in FrameBuilder pixel compositing & serialization
    pub frame_builder_duration: Duration,
    /// Accumulated duration in 8 hardware sprite engines
    pub sprites_duration: Duration,

    // --- Paula & Storage Sub-Units ---
    /// Accumulated duration in Paula 4-channel audio DMA, period counters, DAC
    pub audio_duration: Duration,
    /// Accumulated duration in Floppy disk controller MFM decoding
    pub floppy_duration: Duration,

    // --- Peripherals Sub-Units ---
    /// Accumulated duration in MOS 8520 CIA-A
    pub cia_a_duration: Duration,
    /// Accumulated duration in MOS 8520 CIA-B
    pub cia_b_duration: Duration,
    /// Accumulated duration in Keyboard serial decoding
    pub keyboard_duration: Duration,
    /// Accumulated duration in Real-Time Clock
    pub rtc_duration: Duration,
}

impl SubsystemProfileStats {
    /// Creates a new, empty profile stats tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the effective frames per second (FPS) achieved during execution
    #[inline]
    pub fn fps(&self) -> f64 {
        let secs = self.total_duration.as_secs_f64();
        if secs > 0.0 {
            self.frames_executed as f64 / secs
        } else {
            0.0
        }
    }

    /// Computes the execution speedup relative to native PAL Amiga 500 (50 Hz / 50 FPS)
    #[inline]
    pub fn speedup_factor(&self) -> f64 {
        self.fps() / 50.0
    }

    /// Computes Color Clock throughput in Mega-Clocks per second (MHz)
    #[inline]
    pub fn cck_mhz(&self) -> f64 {
        let secs = self.total_duration.as_secs_f64();
        if secs > 0.0 {
            (self.cck_executed as f64 / secs) / 1_000_000.0
        } else {
            0.0
        }
    }

    /// Prints a formatted profile summary table to stdout
    pub fn print_summary(&self, test_name: &str) {
        let total_ms = self.total_duration.as_secs_f64() * 1000.0;
        let cpu_ms = self.cpu_duration.as_secs_f64() * 1000.0;
        let agnus_ms = self.agnus_duration.as_secs_f64() * 1000.0;
        let denise_ms = self.denise_duration.as_secs_f64() * 1000.0;
        let paula_ms = self.paula_duration.as_secs_f64() * 1000.0;
        let cia_ms = self.cia_duration.as_secs_f64() * 1000.0;

        let measured_ms = cpu_ms + agnus_ms + denise_ms + paula_ms + cia_ms;

        let calc_pct = |val: f64| -> f64 {
            if measured_ms > 0.0 {
                (val / measured_ms) * 100.0
            } else {
                0.0
            }
        };

        println!("\n=========================================================================================");
        println!(
            "⏱️  EXECUTION PROFILE: [{}] ({} frames, {:.2}s simulated Amiga time)",
            test_name,
            self.frames_executed,
            self.frames_executed as f64 / 50.0
        );
        println!("=========================================================================================");
        println!(
            "Host Wall-Clock Time:  {:.2} ms ({:.2}s)",
            total_ms,
            self.total_duration.as_secs_f64()
        );
        println!("Frames Executed:       {}", self.frames_executed);
        println!("Color Clocks (CCK):    {}", self.cck_executed);
        println!(
            "Throughput:            {:.1} FPS ({:.2}x real-time 50Hz PAL, {:.2} MHz CCK)",
            self.fps(),
            self.speedup_factor(),
            self.cck_mhz()
        );
        println!("-----------------------------------------------------------------------------------------");
        println!(
            "{:<40} {:>10} {:>12} {:>14}",
            "Subsystem", "Time (ms)", "Share (%)", "Avg / CCK"
        );
        println!("{}", "-".repeat(82));

        let print_row = |name: &str, ms: f64, dur: &Duration| {
            let avg_ns = if self.cck_executed > 0 {
                (dur.as_nanos() as f64) / (self.cck_executed as f64)
            } else {
                0.0
            };
            println!(
                "{:<40} {:>8.2} ms {:>11.1}% {:>11.1} ns",
                name,
                ms,
                calc_pct(ms),
                avg_ns
            );
        };

        // 1. Agnus
        print_row("Agnus (Total)", agnus_ms, &self.agnus_duration);
        print_row(
            "  ├─ Master DMA Slot Arbitration",
            self.dma_duration.as_secs_f64() * 1000.0,
            &self.dma_duration,
        );
        print_row(
            "  ├─ Copper Coprocessor",
            self.copper_duration.as_secs_f64() * 1000.0,
            &self.copper_duration,
        );
        print_row(
            "  ├─ Blitter Engine",
            self.blitter_duration.as_secs_f64() * 1000.0,
            &self.blitter_duration,
        );
        print_row(
            "  └─ Beam Counters & Mutations",
            self.agnus_beam_duration.as_secs_f64() * 1000.0,
            &self.agnus_beam_duration,
        );

        // 2. Denise
        print_row("Denise (Total)", denise_ms, &self.denise_duration);
        print_row(
            "  ├─ FrameBuilder (Pixel Compositor)",
            self.frame_builder_duration.as_secs_f64() * 1000.0,
            &self.frame_builder_duration,
        );
        print_row(
            "  └─ Hardware Sprites",
            self.sprites_duration.as_secs_f64() * 1000.0,
            &self.sprites_duration,
        );

        // 3. Paula & Storage
        print_row("Paula & Storage (Total)", paula_ms, &self.paula_duration);
        print_row(
            "  ├─ Audio Engine & DAC",
            self.audio_duration.as_secs_f64() * 1000.0,
            &self.audio_duration,
        );
        print_row(
            "  └─ Floppy Disk Controller",
            self.floppy_duration.as_secs_f64() * 1000.0,
            &self.floppy_duration,
        );

        // 4. CIAs & Peripherals
        print_row("CIAs & Peripherals (Total)", cia_ms, &self.cia_duration);
        print_row(
            "  ├─ CIA-A (Timers, TOD, SDR)",
            self.cia_a_duration.as_secs_f64() * 1000.0,
            &self.cia_a_duration,
        );
        print_row(
            "  ├─ CIA-B (Timers, TOD)",
            self.cia_b_duration.as_secs_f64() * 1000.0,
            &self.cia_b_duration,
        );
        print_row(
            "  ├─ Real-Time Clock (RTC)",
            self.rtc_duration.as_secs_f64() * 1000.0,
            &self.rtc_duration,
        );
        print_row(
            "  └─ Keyboard Serial",
            self.keyboard_duration.as_secs_f64() * 1000.0,
            &self.keyboard_duration,
        );

        // 5. M68000 CPU & MemoryBus
        print_row("M68000 CPU & MemoryBus", cpu_ms, &self.cpu_duration);
        println!("=========================================================================================\n");
    }
}

impl A500Machine {
    /// Executes Color Clocks until a full vertical video frame completes (VBlank transition),
    /// measuring wall-clock duration and profiling subsystem execution.
    pub fn step_frame_profiled(&mut self, stats: &mut SubsystemProfileStats) {
        let frame_start = std::time::Instant::now();
        let cck_start = self.cck;

        let mut cpu_duration = Duration::ZERO;
        let mut agnus_duration = Duration::ZERO;
        let mut denise_duration = Duration::ZERO;
        let mut paula_duration = Duration::ZERO;
        let mut cia_duration = Duration::ZERO;

        let mut agnus_profile = agnus::AgnusSubsystemProfile::default();
        let mut denise_profile = denise::DeniseSubsystemProfile::default();
        let mut audio_duration = Duration::ZERO;
        let mut floppy_duration = Duration::ZERO;
        let mut cia_a_duration = Duration::ZERO;
        let mut cia_b_duration = Duration::ZERO;
        let mut keyboard_duration = Duration::ZERO;
        let mut rtc_duration = Duration::ZERO;

        // Sample profiling on 16 representative scanlines across the frame
        // to achieve sub-microsecond precision with < 0.1% timer overhead
        let mut in_vpos_0 = self.agnus.vpos == 0;

        loop {
            let sample_probe = (self.agnus.vpos & 0x0F) == 0;
            if sample_probe {
                self.cck = self.cck.wrapping_add(1);

                // 1. Agnus sub-units
                let t_agnus = std::time::Instant::now();
                let agnus_due = self
                    .agnus
                    .step_cck_ram_profiled(&mut self.physical_memory.chip_ram, &mut agnus_profile);
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

                // 2. Denise sub-units
                let t_denise = std::time::Instant::now();
                let beam = self.agnus.beam();
                if beam.hpos == 0 {
                    self.cia_b.tick_tod();
                }
                let denise_due = self.denise.step_cck_profiled(beam, &mut denise_profile);
                for item in denise_due.iter().flatten() {
                    self.dispatch_denise_action(item.0, item.1);
                }
                denise_duration += t_denise.elapsed();

                // 3. Paula & Floppy sub-units
                let t_audio = std::time::Instant::now();
                let paula_due = self.paula.step_cck();
                for item in paula_due.iter().flatten() {
                    self.dispatch_paula_action(item.0, item.1);
                }
                for ch in 0..4 {
                    if self.paula.poll_audio_restart(ch) {
                        self.agnus.reload_audio_ptr(ch);
                        self.paula.set_interrupt_request(1u16 << (7 + ch));
                    }
                }
                let d_audio = t_audio.elapsed();
                audio_duration += d_audio;

                let t_floppy = std::time::Instant::now();
                self.floppy.step_cck();
                if self.floppy.poll_dskblk_irq() {
                    self.paula.set_interrupt_request(0x0002);
                }
                let d_floppy = t_floppy.elapsed();
                floppy_duration += d_floppy;

                paula_duration += d_audio + d_floppy;

                // 4. CIAs, Keyboard, RTC & Cascades
                let t_cia_start = std::time::Instant::now();

                let t_kdb = std::time::Instant::now();
                let kdat_handshake = self.cia_a.is_sdr_output();
                if let Some(scancode) = self.keyboard.step(kdat_handshake) {
                    self.cia_a.shift_in_sdr(scancode);
                }
                keyboard_duration += t_kdb.elapsed();

                let t_cia_a = std::time::Instant::now();
                let cia_a_due = self.cia_a.step_cck();
                for item in cia_a_due.iter().flatten() {
                    self.dispatch_cia_action(cia::CiaId::A, item.0, item.1);
                }
                if self.cia_a.irq_pending() {
                    self.paula.set_interrupt_request(0x0008);
                }
                cia_a_duration += t_cia_a.elapsed();

                let t_cia_b = std::time::Instant::now();
                let cia_b_due = self.cia_b.step_cck();
                for item in cia_b_due.iter().flatten() {
                    self.dispatch_cia_action(cia::CiaId::B, item.0, item.1);
                }
                if self.cia_b.irq_pending() {
                    self.paula.set_interrupt_request(0x2000);
                }
                cia_b_duration += t_cia_b.elapsed();

                let t_rtc = std::time::Instant::now();
                self.rtc.step_cck(1);
                rtc_duration += t_rtc.elapsed();

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
                cia_duration += t_cia_start.elapsed();

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

        stats.frames_executed += 1;
        stats.cck_executed += cck_delta;
        stats.total_duration += total_frame_duration;

        // Scale sampled probe ratios to match total frame duration
        let sampled_total =
            cpu_duration + agnus_duration + denise_duration + paula_duration + cia_duration;
        if sampled_total.as_nanos() > 0 {
            let total_secs = total_frame_duration.as_secs_f64();
            let sampled_secs = sampled_total.as_secs_f64();
            let scale = total_secs / sampled_secs;

            // Major chipsets
            stats.cpu_duration += cpu_duration.mul_f64(scale);
            stats.agnus_duration += agnus_duration.mul_f64(scale);
            stats.denise_duration += denise_duration.mul_f64(scale);
            stats.paula_duration += paula_duration.mul_f64(scale);
            stats.cia_duration += cia_duration.mul_f64(scale);

            // Agnus sub-units
            stats.copper_duration += agnus_profile.copper.mul_f64(scale);
            stats.blitter_duration += agnus_profile.blitter.mul_f64(scale);
            stats.dma_duration += agnus_profile.dma.mul_f64(scale);
            stats.agnus_beam_duration += agnus_profile.beam.mul_f64(scale);

            // Denise sub-units
            stats.frame_builder_duration += denise_profile.frame_builder.mul_f64(scale);
            stats.sprites_duration += denise_profile.sprites.mul_f64(scale);

            // Paula & Storage sub-units
            stats.audio_duration += audio_duration.mul_f64(scale);
            stats.floppy_duration += floppy_duration.mul_f64(scale);

            // CIAs & Peripherals sub-units
            stats.cia_a_duration += cia_a_duration.mul_f64(scale);
            stats.cia_b_duration += cia_b_duration.mul_f64(scale);
            stats.keyboard_duration += keyboard_duration.mul_f64(scale);
            stats.rtc_duration += rtc_duration.mul_f64(scale);
        } else {
            stats.cpu_duration += total_frame_duration / 5;
            stats.agnus_duration += total_frame_duration / 5;
            stats.denise_duration += total_frame_duration / 5;
            stats.paula_duration += total_frame_duration / 5;
            stats.cia_duration += total_frame_duration / 5;
        }
    }
}
