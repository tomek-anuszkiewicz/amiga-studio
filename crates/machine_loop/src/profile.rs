//! Amiga 500 Machine Loop Execution Profiler
//!
//! Provides zero-cost execution metrics, FPS tracking, and subsystem timing probes.

use std::time::Duration;

/// Comprehensive subsystem execution statistics and timing breakdown
#[derive(Debug, Clone, Default)]
pub struct SubsystemProfileStats {
    /// Total elapsed wall-clock time across profiled frames
    pub total_duration: Duration,
    /// Accumulated duration spent in M68000 CPU execution & bus routing
    pub cpu_duration: Duration,
    /// Accumulated duration spent in Agnus (DMA slot scheduling, Copper, Blitter)
    pub agnus_duration: Duration,
    /// Accumulated duration spent in Denise (Sprites, FrameBuilder pixel compositing)
    pub denise_duration: Duration,
    /// Accumulated duration spent in Paula & Floppy controller
    pub paula_duration: Duration,
    /// Accumulated duration spent in CIAs A & B, Keyboard, and RTC
    pub cia_duration: Duration,
    /// Total number of full vertical video frames executed
    pub frames_executed: u64,
    /// Total number of Color Clock cycles stepped
    pub cck_executed: u64,
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
            "{:<36} {:>10} {:>12} {:>14}",
            "Subsystem", "Time (ms)", "Share (%)", "Avg / CCK"
        );
        println!("{}", "-".repeat(78));

        let print_row = |name: &str, ms: f64, dur: &Duration| {
            let avg_ns = if self.cck_executed > 0 {
                (dur.as_nanos() as f64) / (self.cck_executed as f64)
            } else {
                0.0
            };
            println!(
                "{:<36} {:>8.2} ms {:>11.1}% {:>11.1} ns",
                name,
                ms,
                calc_pct(ms),
                avg_ns
            );
        };

        print_row("M68000 CPU & MemoryBus", cpu_ms, &self.cpu_duration);
        print_row(
            "Denise Video & FrameBuilder",
            denise_ms,
            &self.denise_duration,
        );
        print_row(
            "Agnus (Copper, Blitter, DMA)",
            agnus_ms,
            &self.agnus_duration,
        );
        print_row("Paula Audio & Floppy", paula_ms, &self.paula_duration);
        print_row("CIAs (A/B), Keyboard & RTC", cia_ms, &self.cia_duration);
        println!("=========================================================================================\n");
    }
}
