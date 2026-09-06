//! Sub-cycle timing and 2-phase CCK bus arbitration (CCK1 and CCK2)

use super::MemoryBus;

/// Result of an M68000 bus transaction phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
