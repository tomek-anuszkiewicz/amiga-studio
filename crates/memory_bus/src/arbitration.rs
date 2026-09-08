//! Sub-cycle timing and Color Clock (CCK) phases
//!
//! Models the Amiga 2-phase Color Clock execution model (CCK1 and CCK2)
//! corresponding to the 4-clock M68000 CPU bus cycle.

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
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CckPhase {
    /// Color Clock Phase 1 (CPU S0–S3): Address output, _AS strobe, contention check
    #[default]
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

/// Bus access transfer size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusAccessSize {
    Byte,
    Word,
}

/// Result of a physical bus access attempt (read or write)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusResult<T> {
    /// Bus access was granted and completed successfully with data (or `()` for write)
    Ready(T),
    /// Bus access stalled due to Agnus DMA cycle stealing / wait state
    WaitState,
}

impl<T> BusResult<T> {
    /// Returns true if the bus access resulted in a wait state
    #[inline(always)]
    pub fn is_wait(&self) -> bool {
        matches!(self, BusResult::WaitState)
    }

    /// Returns true if the bus access completed successfully
    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        matches!(self, BusResult::Ready(_))
    }

    /// Converts `BusResult<T>` to `Option<T>`
    #[inline(always)]
    pub fn ok(self) -> Option<T> {
        match self {
            BusResult::Ready(val) => Some(val),
            BusResult::WaitState => None,
        }
    }

    /// Unwraps the value or returns a default fallback
    #[inline(always)]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            BusResult::Ready(val) => val,
            BusResult::WaitState => default,
        }
    }
}

impl MemoryBus {
    /// Helper to identify whether an address targets Chip RAM or contention-affected Slow RAM
    #[inline]
    pub fn is_chip_ram_target(&self, addr: u32) -> bool {
        let addr = addr & 0x00FF_FFFF;
        if self.low_memory_overlay && addr < 0x080000 {
            // Overlay maps low memory to Kickstart ROM (ROM has zero contention)
            return false;
        }
        // In synthetic test harnesses (SingleStepTests flat memory), all test memory is treated as Chip RAM
        if self.test_memory.is_some() {
            return true;
        }
        // Chip RAM range: $000000-$07FFFF (or $000000-$0FFFFF for 1MB)
        // Slow RAM range: $C00000-$C7FFFF (handled via Gary; subject to shared Agnus bus contention)
        addr < (self.chip_ram.len() as u32) || (0xC00000..=0xC7FFFF).contains(&addr)
    }
}
