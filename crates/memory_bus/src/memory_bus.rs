//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::Cia;
use config::{custom_reg, OPEN_BUS_BYTE, OPEN_BUS_WORD};
use denise::Denise;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
use rtc::RtcMsm6242b;

pub use physical_memory;

mod cia_registers;
mod custom_writes;

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

    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT, DSKBYTR)
    #[inline]
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & Self::CUSTOM_REG_OFFSET_MASK;
        match offset {
            // Early DMA reads are not ordinary CPU-readable registers.
            custom_reg::agnus::BLTDDAT | custom_reg::paula::DSKDATR => OPEN_BUS_WORD,
            custom_reg::agnus::DMACONR => self.agnus.dmaconr(),
            custom_reg::agnus::VPOSR => self.agnus.vposr(),
            custom_reg::agnus::VHPOSR => self.agnus.vhposr(),
            custom_reg::denise::JOY0DAT => self.denise.joy0dat(),
            custom_reg::denise::JOY1DAT => self.denise.joy1dat(),
            custom_reg::denise::CLXDAT => self.denise.clxdat(),
            custom_reg::paula::ADKCONR => self.paula.adkconr(),
            custom_reg::paula::POT0DAT => self.paula.pot0dat(),
            custom_reg::paula::POT1DAT => self.paula.pot1dat(),
            custom_reg::paula::POTGOR => self.paula.potgor(),
            custom_reg::paula::SERDATR => self.paula.serdatr(),
            custom_reg::paula::DSKBYTR => self.paula.read_dskbytr(),
            custom_reg::paula::INTENAR => self.paula.intenar(),
            custom_reg::paula::INTREQR => self.paula.intreqr(),
            custom_reg::agnus::COPJMP1 => {
                self.agnus.strobe_copjmp1();
                OPEN_BUS_WORD
            }
            custom_reg::agnus::COPJMP2 => {
                self.agnus.strobe_copjmp2();
                OPEN_BUS_WORD
            }
            _ => OPEN_BUS_WORD,
        }
    }

    /// Reads a 16-bit custom register without side-effects for debugging inspection
    #[inline]
    pub fn read_custom_word_debug(&self, offset: u16) -> u16 {
        let offset = offset & Self::CUSTOM_REG_OFFSET_MASK;
        match offset {
            custom_reg::agnus::DMACONR => self.agnus.dmaconr_debug(),
            custom_reg::agnus::VPOSR => self.agnus.vposr_debug(),
            custom_reg::agnus::VHPOSR => self.agnus.vhposr_debug(),
            custom_reg::denise::JOY0DAT => self.denise.joy0dat_debug(),
            custom_reg::denise::JOY1DAT => self.denise.joy1dat_debug(),
            custom_reg::denise::CLXDAT => self.denise.clxdat_debug(),
            custom_reg::paula::ADKCONR => self.paula.adkconr_debug(),
            custom_reg::paula::POT0DAT => self.paula.pot0dat_debug(),
            custom_reg::paula::POT1DAT => self.paula.pot1dat_debug(),
            custom_reg::paula::POTGOR => self.paula.potgor_debug(),
            custom_reg::paula::SERDATR => self.paula.serdatr_debug(),
            custom_reg::paula::DSKBYTR => self.paula.peek_dskbytr(),
            custom_reg::paula::INTENAR => self.paula.intenar_debug(),
            custom_reg::paula::INTREQR => self.paula.intreqr_debug(),
            _ => OPEN_BUS_WORD,
        }
    }

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
