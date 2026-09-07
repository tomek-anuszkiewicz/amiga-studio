//! Sub-cycle timing and 2-phase CCK bus arbitration (CCK1 and CCK2)

use super::MemoryBus;
use serde::{Deserialize, Serialize};

/// M68000 Function Code lines (FC0-FC2)
pub mod function_code {
    pub const USER_DATA: u8 = 1;
    pub const USER_PROGRAM: u8 = 2;
    pub const SUPERVISOR_DATA: u8 = 5;
    pub const SUPERVISOR_PROGRAM: u8 = 6;
    pub const CPU_SPACE: u8 = 7;
}

/// Color Clock (CCK) sub-cycle phase of the 4-clock M68000 bus cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CckPhase {
    /// Color Clock Phase 1 (CPU S0–S3): Address output, _AS strobe, contention arbitration
    Cck1,
    /// Color Clock Phase 2 (CPU S4–S7): Data latch/write commit, _DTACK acknowledgement
    Cck2,
}

impl CckPhase {
    /// Advances to the alternating Color Clock phase
    #[inline]
    pub fn next(self) -> Self {
        match self {
            CckPhase::Cck1 => CckPhase::Cck2,
            CckPhase::Cck2 => CckPhase::Cck1,
        }
    }
}

impl Default for CckPhase {
    fn default() -> Self {
        CckPhase::Cck1
    }
}

/// Bus access transfer size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusAccessSize {
    Byte,
    Word,
}

/// Structured M68000 bus cycle representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusCycle {
    /// 24-bit physical memory address
    pub addr: u32,
    /// 16-bit data bus transfer value
    pub data: u16,
    /// Access size (Byte or Word)
    pub size: BusAccessSize,
    /// Function Code lines (FC0-FC2)
    pub fc: u8,
    /// Read (true) vs Write (false) direction
    pub is_read: bool,
    /// Upper Data Strobe (_UDS, asserts for bits 15..8 / even byte)
    pub uds: bool,
    /// Lower Data Strobe (_LDS, asserts for bits 7..0 / odd byte)
    pub lds: bool,
}

impl BusCycle {
    /// Constructs a new Read bus cycle with automatically determined UDS/LDS strobes
    #[inline]
    pub fn new_read(addr: u32, size: BusAccessSize, fc: u8) -> Self {
        let (uds, lds) = match size {
            BusAccessSize::Word => (true, true),
            BusAccessSize::Byte => {
                if (addr & 1) == 0 {
                    (true, false)
                } else {
                    (false, true)
                }
            }
        };
        Self {
            addr: addr & 0x00FF_FFFF,
            data: 0,
            size,
            fc,
            is_read: true,
            uds,
            lds,
        }
    }

    /// Constructs a new Write bus cycle with automatically determined UDS/LDS strobes
    #[inline]
    pub fn new_write(addr: u32, data: u16, size: BusAccessSize, fc: u8) -> Self {
        let (uds, lds) = match size {
            BusAccessSize::Word => (true, true),
            BusAccessSize::Byte => {
                if (addr & 1) == 0 {
                    (true, false)
                } else {
                    (false, true)
                }
            }
        };
        Self {
            addr: addr & 0x00FF_FFFF,
            data,
            size,
            fc,
            is_read: false,
            uds,
            lds,
        }
    }
}

/// Result of an M68000 bus transaction phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryBusResult {
    /// Phase 1 completed; proceed to Phase 2
    Phase1Ready,
    /// Transaction completed successfully (carries 16-bit word or zero-extended 8-bit byte on read, 0 on write)
    Ready(u16),
    /// Target bus is currently occupied by DMA (CPU must hold state and retry/insert wait states)
    Blocked,
}

impl MemoryBus {
    /// Phase 1 Read (CCK1 / S0-S3): Checks bus contention and latches data
    pub fn read_phase1(&mut self, addr: u32, is_byte: bool, high_byte: bool) -> MemoryBusResult {
        let addr = addr & 0x00FF_FFFF;

        // If target region is Chip RAM (or Slow RAM with shared contention) and bus is blocked by Agnus DMA
        if self.is_chip_ram_target(addr) && self.chip_ram_blocked {
            return MemoryBusResult::Blocked;
        }

        // Perform memory read directly into read_latch
        let word = self.read_word_internal(addr);
        self.read_latch = if is_byte {
            if high_byte {
                (word >> 8) & 0xFF
            } else {
                word & 0xFF
            }
        } else {
            word
        };

        MemoryBusResult::Phase1Ready
    }

    /// Phase 2 Read (CCK2 / S4-S7): Delivers data from read_latch without contention
    #[inline]
    pub fn read_phase2(&mut self, _addr: u32) -> MemoryBusResult {
        // CPU reads safely from read_latch while physical bus is freed for custom chip DMA
        MemoryBusResult::Ready(self.read_latch)
    }

    /// Phase 1 Write (CCK1 / S0-S3): CPU presents address and data onto pins
    #[inline]
    pub fn write_phase1(&mut self, _addr: u32, data: u16) -> MemoryBusResult {
        self.pending_write_data = data;
        MemoryBusResult::Phase1Ready
    }

    /// Phase 2 Write (CCK2 / S4-S7): Bus commits write to physical memory unless Gary withholds _DTACK
    pub fn write_phase2(
        &mut self,
        addr: u32,
        data: u16,
        is_byte: bool,
        _high_byte: bool,
    ) -> MemoryBusResult {
        let addr = addr & 0x00FF_FFFF;

        // If target region is Chip RAM and bus is blocked by DMA, Gary withholds _DTACK
        if self.is_chip_ram_target(addr) && self.chip_ram_blocked {
            return MemoryBusResult::Blocked;
        }

        // Commit byte or word to physical memory
        if is_byte {
            let byte_val = (data & 0xFF) as u8;
            self.write_byte_internal(addr, byte_val);
        } else {
            self.write_word_internal(addr, data);
        }

        MemoryBusResult::Ready(0)
    }

    /// Begin a structured bus cycle (CCK1 / S0-S3): checks contention and presents bus signals
    pub fn begin_cycle(&mut self, cycle: &mut BusCycle) -> MemoryBusResult {
        let addr = cycle.addr & 0x00FF_FFFF;
        if cycle.is_read {
            if self.is_chip_ram_target(addr) && self.chip_ram_blocked {
                return MemoryBusResult::Blocked;
            }
            self.read_latch = self.read_word_internal(addr & !1);
            MemoryBusResult::Phase1Ready
        } else {
            self.pending_write_data = cycle.data;
            MemoryBusResult::Phase1Ready
        }
    }

    /// End a structured bus cycle (CCK2 / S4-S7): latches read data or commits write
    pub fn end_cycle(&mut self, cycle: &mut BusCycle) -> MemoryBusResult {
        let addr = cycle.addr & 0x00FF_FFFF;
        if cycle.is_read {
            cycle.data = match cycle.size {
                BusAccessSize::Word => self.read_latch,
                BusAccessSize::Byte => {
                    if cycle.uds {
                        (self.read_latch >> 8) & 0xFF
                    } else {
                        self.read_latch & 0xFF
                    }
                }
            };
            MemoryBusResult::Ready(cycle.data)
        } else {
            if self.is_chip_ram_target(addr) && self.chip_ram_blocked {
                return MemoryBusResult::Blocked;
            }
            match cycle.size {
                BusAccessSize::Byte => {
                    self.write_byte_internal(addr, (cycle.data & 0xFF) as u8);
                }
                BusAccessSize::Word => {
                    self.write_word_internal(addr, cycle.data);
                }
            }
            MemoryBusResult::Ready(0)
        }
    }

    /// Helper to identify whether an address targets Chip RAM or contention-affected Slow RAM
    #[inline]
    pub fn is_chip_ram_target(&self, addr: u32) -> bool {
        let addr = addr & 0x00FF_FFFF;
        if self.low_memory_overlay && addr < 0x080000 {
            // Overlay maps low memory to Kickstart ROM (ROM has zero contention)
            return false;
        }
        // Chip RAM range: $000000-$07FFFF (or $000000-$0FFFFF for 1MB)
        // Slow RAM range: $C00000-$C7FFFF (handled via Gary; subject to shared Agnus bus contention)
        addr < (self.chip_ram.len() as u32) || (addr >= 0xC00000 && addr <= 0xC7FFFF)
    }
}
