//! 24-Bit Address Bus Protocol and Transfer Semantics
//!
//! Encapsulates cycle-exact bus transfer results (`BusResult`) and the core
//! `AddressBus` trait implemented by both physical memory backplanes and test harnesses.

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

/// Abstract 24-bit address bus interface
///
/// Enables the M68000 CPU core to operate interchangeably with either the
/// production Amiga 500 backplane (`MemoryBus`) or synthetic test harnesses (`TestMemoryBus`).
pub trait AddressBus {
    /// Reads a single byte from the 24-bit address space
    fn read_byte(&mut self, addr: u32) -> BusResult<u8>;

    /// Reads a 16-bit word from the 24-bit address space (must be even address)
    fn read_word(&mut self, addr: u32) -> BusResult<u16>;

    /// Writes a single byte to the 24-bit address space
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()>;

    /// Writes a 16-bit word to the 24-bit address space (must be even address)
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()>;

    /// Non-intrusive debug word read (does not advance bus state or trigger wait states)
    fn read_word_debug(&self, addr: u32) -> u16;
}
