//! CPU-only benchmark clock ownership; peers remain unclocked as before.
use memory_bus::MemoryBus;
use physical_memory::{AddressBus, BusResult};

pub(super) struct BenchmarkPlatformBus<'a>(pub(super) MemoryBus<'a>);

impl AddressBus for BenchmarkPlatformBus<'_> {
    fn begin_cpu_cck(&mut self, _: u64, pc: u32) -> u64 {
        let cck = self.0.mem.memory_cck().wrapping_add(1);
        self.0.mem.begin_cck(cck, pc);
        self.0.cck = cck;
        cck
    }
    fn read_byte(&mut self, address: u32) -> BusResult<u8> {
        self.0.read_byte(address)
    }
    fn read_word(&mut self, address: u32) -> BusResult<u16> {
        self.0.read_word(address)
    }
    fn write_byte(&mut self, address: u32, value: u8) -> BusResult<()> {
        self.0.write_byte(address, value)
    }
    fn write_tas_byte(&mut self, address: u32, value: u8) -> BusResult<()> {
        self.0.write_tas_byte(address, value)
    }
    fn write_word(&mut self, address: u32, value: u16) -> BusResult<()> {
        self.0.write_word(address, value)
    }
    fn read_word_debug(&self, address: u32) -> u16 {
        self.0.read_word_debug(address)
    }
}
