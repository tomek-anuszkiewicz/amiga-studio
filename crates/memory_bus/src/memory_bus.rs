//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::Cia;
use config::OPEN_BUS_BYTE;
use denise::Denise;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
use rtc::RtcMsm6242b;

pub use physical_memory;

mod cia_registers;
mod custom_registers;

/// Zero-cost stack-allocated router implementing `AddressBus` across all subsystems
#[derive(Debug)]
pub struct MemoryBus<'a> {
    pub mem: &'a mut PhysicalMemory,
    pub agnus: &'a mut Agnus,
    pub denise: &'a mut Denise,
    pub paula: &'a mut Paula,
    pub cia_a: &'a mut Cia,
    pub cia_b: &'a mut Cia,
    pub rtc: &'a mut RtcMsm6242b,
}

impl<'a> MemoryBus<'a> {
    const PHYSICAL_ADDRESS_MASK: u32 = 0x00FF_FFFF;

    /// Physical memory map constants decoded by Gary logic
    pub const BANK_CIA: u8 = 0xBF;
    pub const BANK_RTC: u8 = 0xDC;
    pub const BANK_CUSTOM: u8 = 0xDF;

    pub const RTC_START: u32 = 0xDC0000;
    pub const RTC_END: u32 = 0xDC003F;

    pub const CIA_REG_ADDRESS_MASK: u32 = 0xFFFF01;
    pub const CUSTOM_REG_OFFSET_MASK: u16 = 0x01FE;
    const CIAA_PA_OVL: u8 = 0x01;

    /// Writes an arbitrary byte slice directly to physical memory for debugger/test injection
    #[inline(always)]
    pub fn write_bytes_debug(&mut self, addr: u32, data: &[u8]) -> usize {
        self.mem.write_bytes_debug(addr, data)
    }
}

impl<'a> AddressBus for MemoryBus<'a> {
    #[inline(always)]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => {
                let word =
                    self.read_custom_word((addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16);
                let byte = if (addr & 1) == 0 {
                    (word >> 8) as u8
                } else {
                    (word & 0xFF) as u8
                };
                BusResult::Ready(byte)
            }
            Self::BANK_CIA => self.read_cia_byte(addr),
            Self::BANK_RTC => {
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    BusResult::Ready(self.rtc.read_byte(addr))
                } else {
                    BusResult::Ready(OPEN_BUS_BYTE)
                }
            }
            _ => self.mem.read_byte(addr),
        }
    }

    #[inline(always)]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => BusResult::Ready(
                self.read_custom_word((addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16),
            ),
            Self::BANK_CIA => {
                let b0 = match self.read_cia_byte(addr) {
                    BusResult::Ready(value) => value,
                    BusResult::WaitState => return BusResult::WaitState,
                };
                let b1 = match self.read_cia_byte(addr.wrapping_add(1)) {
                    BusResult::Ready(value) => value,
                    BusResult::WaitState => return BusResult::WaitState,
                };
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            Self::BANK_RTC => {
                let b0 = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    OPEN_BUS_BYTE
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    OPEN_BUS_BYTE
                };
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            _ => self.mem.read_word(addr),
        }
    }

    #[inline(always)]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => {
                if self.write_custom_byte(addr, val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            Self::BANK_CIA => self.write_cia_byte(addr, val),
            Self::BANK_RTC => {
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, val);
                }
                BusResult::Ready(())
            }
            _ => self.mem.write_byte(addr, val),
        }
    }

    #[inline]
    fn write_tas_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = self.mem.bank_map[(addr >> 16) as usize].bank;
        if matches!(bank, MemoryBank::ChipRam | MemoryBank::SlowRam) {
            if self.mem.chip_ram_blocked {
                BusResult::WaitState
            } else {
                BusResult::Ready(())
            }
        } else {
            self.write_byte(addr, val)
        }
    }

    #[inline(always)]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => {
                let offset = (addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16;
                if self.write_custom_word(offset, val) {
                    BusResult::Ready(())
                } else {
                    BusResult::WaitState
                }
            }
            Self::BANK_CIA => {
                let bytes = val.to_be_bytes();
                if self.write_cia_byte(addr, bytes[0]).is_wait() {
                    return BusResult::WaitState;
                }
                self.write_cia_byte(addr.wrapping_add(1), bytes[1])
            }
            Self::BANK_RTC => {
                let bytes = val.to_be_bytes();
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, bytes[0]);
                }
                let next_addr = addr.wrapping_add(1);
                if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.write_byte(next_addr, bytes[1]);
                }
                BusResult::Ready(())
            }
            _ => self.mem.write_word(addr, val),
        }
    }

    #[inline(always)]
    fn read_word_debug(&self, addr: u32) -> u16 {
        let addr = addr & Self::PHYSICAL_ADDRESS_MASK;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => {
                self.read_custom_word_debug((addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16)
            }
            Self::BANK_CIA => {
                let b0 = self.read_cia_byte_debug(addr);
                let b1 = self.read_cia_byte_debug(addr.wrapping_add(1));
                u16::from_be_bytes([b0, b1])
            }
            Self::BANK_RTC => {
                let b0 = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    OPEN_BUS_BYTE
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    OPEN_BUS_BYTE
                };
                u16::from_be_bytes([b0, b1])
            }
            _ => self.mem.read_word_debug(addr),
        }
    }
}
