//! Pure machine snapshot values, independent of persistence formats and host I/O.

use config::A500Config;
use cpu::CpuState;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};

/// Master state container representing a full Amiga 500 machine snapshot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A500State {
    /// Captured keyboard reset level awaiting the next CCK.
    pub keyboard_reset_asserted: bool,
    /// Master monotonic Color Clock counter
    pub cck: u64,
    /// Active hardware configuration model
    pub config: A500Config,
    /// Motorola 68000 CPU register and micro-execution state
    pub cpu: CpuState,
    /// 24-bit physical memory buffers and overlay flag
    pub physical_memory: PhysicalMemory,
    /// OKI MSM6242B Real-Time Clock
    pub rtc: rtc::RtcMsm6242b,
    /// Agnus (beam counters, copper, blitter, dma, chip RAM bus lock)
    pub agnus: agnus::Agnus,
    /// Denise (video control, sprites, frame builder, color palette, collisions)
    pub denise: denise::Denise,
    /// Paula (audio channels, serial UART, interrupt multiplexer)
    pub paula: paula::Paula,
    /// MOS 8520 CIA-A
    pub cia_a: cia::Cia,
    /// MOS 8520 CIA-B
    pub cia_b: cia::Cia,
    /// 3.5" DD Floppy controller and drive units
    pub floppy: floppy::FloppyController,
    /// MOS 6500/1 keyboard microcontroller
    pub keyboard: keyboard::Keyboard,
    /// Dual Atari 9-pin controller game ports
    pub game_ports: game_ports::GamePorts,
}
