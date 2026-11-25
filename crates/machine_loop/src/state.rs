//! Pure machine snapshot values, independent of persistence formats and host I/O.

use config::A500Config;
use cpu::CpuState;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};

/// Master state container representing a full Amiga 500 machine snapshot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A500State<
    Memory = PhysicalMemory,
    Display = denise::Denise,
    Disk = floppy::FloppyController,
> {
    /// Captured keyboard reset level awaiting the next CCK.
    pub keyboard_reset_asserted: bool,
    pub disk_activity_generation: u64,
    /// Master monotonic Color Clock counter
    pub cck: u64,
    pub config: A500Config,
    pub cpu: CpuState,
    pub physical_memory: Memory,
    pub agnus: agnus::Agnus,
    pub denise: Display,
    pub paula: paula::Paula,
    pub cia_a: cia::Cia,
    pub cia_b: cia::Cia,
    pub floppy: Disk,
    pub keyboard: keyboard::Keyboard,
    pub game_ports: game_ports::GamePorts,
}

impl<Memory, Display, Disk> A500State<Memory, Display, Disk> {
    /// Converts the three component storage layouts used by file persistence.
    pub fn map_storage<M, V, F>(
        self,
        memory: impl FnOnce(Memory) -> M,
        display: impl FnOnce(Display) -> V,
        disk: impl FnOnce(Disk) -> F,
    ) -> A500State<M, V, F> {
        A500State {
            keyboard_reset_asserted: self.keyboard_reset_asserted,
            disk_activity_generation: self.disk_activity_generation,
            cck: self.cck,
            config: self.config,
            cpu: self.cpu,
            physical_memory: memory(self.physical_memory),
            agnus: self.agnus,
            denise: display(self.denise),
            paula: self.paula,
            cia_a: self.cia_a,
            cia_b: self.cia_b,
            floppy: disk(self.floppy),
            keyboard: self.keyboard,
            game_ports: self.game_ports,
        }
    }
}
