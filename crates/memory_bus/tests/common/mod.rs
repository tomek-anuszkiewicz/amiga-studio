use memory_bus::MemoryBus;
use physical_memory::PhysicalMemory;

/// Explicitly advance the isolated storage/router fixture to another CCK.
pub trait BusCycle {
    fn next_cck(&mut self) -> &mut Self;
}
impl BusCycle for MemoryBus<'_> {
    fn next_cck(&mut self) -> &mut Self {
        self.cck = self.mem.memory_cck().wrapping_add(1);
        self.mem.begin_cck(self.cck, 0);
        self
    }
}
impl BusCycle for PhysicalMemory {
    fn next_cck(&mut self) -> &mut Self {
        self.begin_cck(self.memory_cck().wrapping_add(1), 0);
        self
    }
}
