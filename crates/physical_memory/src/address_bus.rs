//! Memory Bus Abstraction
//!
//! Defines the core `AddressBus` trait implemented by both authentic hardware
//! emulation buses (`MemoryBus`) and synthetic test-runner harnesses (`TestMemoryBus`).

use super::bus_result::BusResult;

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
