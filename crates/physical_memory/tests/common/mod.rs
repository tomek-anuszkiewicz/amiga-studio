use physical_memory::PhysicalMemory;

pub trait MemoryCycle {
    fn next_cck(&mut self) -> &mut Self;
}
impl MemoryCycle for PhysicalMemory {
    fn next_cck(&mut self) -> &mut Self {
        self.begin_cck(self.memory_cck().wrapping_add(1), 0);
        self
    }
}
