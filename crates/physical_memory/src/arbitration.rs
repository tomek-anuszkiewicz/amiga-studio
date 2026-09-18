//! Bus arbitration, function codes, access sizes, and bus transfer results
//!
//! Encapsulates transfer qualifiers (FC0-FC2), operand sizes (Byte, Word),
//! and passive bus access result (`BusResult`) for Chip RAM DMA contention.

use serde::{Deserialize, Serialize};

/// M68000 Function Code lines (FC0-FC2)
pub mod function_code {
    pub const USER_DATA: u8 = 1;
    pub const USER_PROGRAM: u8 = 2;
    pub const SUPERVISOR_DATA: u8 = 5;
    pub const SUPERVISOR_PROGRAM: u8 = 6;
    pub const CPU_SPACE: u8 = 7;
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
