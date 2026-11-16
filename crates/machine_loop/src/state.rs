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
    pub config: A500Config,
    pub cpu: CpuState,
    pub physical_memory: PhysicalMemory,
    pub agnus: agnus::Agnus,
    pub denise: denise::Denise,
    pub paula: paula::Paula,
    pub cia_a: cia::Cia,
    pub cia_b: cia::Cia,
    pub floppy: floppy::FloppyController,
    pub keyboard: keyboard::Keyboard,
    pub game_ports: game_ports::GamePorts,
}
