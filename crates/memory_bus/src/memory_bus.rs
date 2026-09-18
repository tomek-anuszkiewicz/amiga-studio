//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::Cia;
use config::custom_reg;
use denise::Denise;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, PhysicalMemory};
use rtc::RtcMsm6242b;

pub use physical_memory;

/// Physical memory map constants decoded by Gary logic
pub const BANK_CIA: u8 = 0xBF;
pub const BANK_RTC: u8 = 0xDC;
pub const BANK_CUSTOM: u8 = 0xDF;

pub const CIA_B_START: u32 = 0xBFD000;
pub const CIA_B_END: u32 = 0xBFDF00;
pub const CIA_A_START: u32 = 0xBFE001;
pub const CIA_A_END: u32 = 0xBFEF01;

pub const RTC_START: u32 = 0xDC0000;
pub const RTC_END: u32 = 0xDC003F;

pub const CUSTOM_REG_OFFSET_MASK: u16 = 0x01FE;

/// Zero-cost stack-allocated router implementing `AddressBus` across all subsystems
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
    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT, DSKBYTR)
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & CUSTOM_REG_OFFSET_MASK;
        match offset {
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
                0xFFFF
            }
            custom_reg::agnus::COPJMP2 => {
                self.agnus.strobe_copjmp2();
                0xFFFF
            }
            _ => 0xFFFF,
        }
    }

    /// Reads a 16-bit custom register without side-effects for debugging inspection
    pub fn read_custom_word_debug(&self, offset: u16) -> u16 {
        let offset = offset & CUSTOM_REG_OFFSET_MASK;
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
            _ => 0xFFFF,
        }
    }

    /// Reads an 8-bit byte from CIA register space ($BF0000..$BFFFFF)
    pub fn read_cia_byte(&mut self, addr: u32) -> u8 {
        // CIA-B ($BFD000-$BFDF00): Even byte addresses (A0 = 0)
        if (CIA_B_START..=CIA_B_END).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_b.read_register(reg);
            }
            return 0xFF;
        }
        // CIA-A ($BFE001-$BFEF01): Odd byte addresses (A0 = 1)
        if (CIA_A_START..=CIA_A_END).contains(&addr) {
            if (addr & 1) == 1 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_a.read_register(reg);
            }
            return 0xFF;
        }
        0xFF
    }

    /// Reads an 8-bit byte from CIA register space without side-effects for debugging inspection
    pub fn read_cia_byte_debug(&self, addr: u32) -> u8 {
        if (CIA_B_START..=CIA_B_END).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_b.read_register_debug(reg);
            }
            return 0xFF;
        }
        if (CIA_A_START..=CIA_A_END).contains(&addr) {
            if (addr & 1) == 1 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_a.read_register_debug(reg);
            }
            return 0xFF;
        }
        0xFF
    }

    /// Writes an 8-bit byte to CIA register space ($BF0000..$BFFFFF)
    pub fn write_cia_byte(&mut self, addr: u32, val: u8) {
        // CIA-B ($BFD000-$BFDF00)
        if (CIA_B_START..=CIA_B_END).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                self.cia_b.write_register(reg, val);
            }
            return;
        }
        // CIA-A ($BFE001-$BFEF01)
        if (CIA_A_START..=CIA_A_END).contains(&addr) && (addr & 1) == 1 {
            let reg = ((addr >> 8) & 0x0F) as u8;
            self.cia_a.write_register(reg, val);
            // CIA-A bit 0 of Port A ($BFE001) controls the low-memory overlay (_OVL)
            if reg == 0 {
                if (val & 0x01) == 0 {
                    self.mem.map_kickstart_to_low_memory();
                } else {
                    self.mem.map_chip_ram_to_low_memory();
                }
            }
        }
    }

    /// Writes a 16-bit word to custom register space with physical propagation delay.
    pub fn write_custom_word(&mut self, offset: u16, val: u16) {
        let offset = offset & CUSTOM_REG_OFFSET_MASK;
        match offset {
            custom_reg::agnus::DMACON => {
                self.agnus.write_register(custom_reg::agnus::DMACON, val);
                self.paula.write_register(custom_reg::paula::DMACON, val);
            }
            custom_reg::denise::BPLCON0 => {
                self.denise.write_register(custom_reg::denise::BPLCON0, val);
                self.agnus.write_register(custom_reg::agnus::BPLCON0, val);
            }
            custom_reg::denise::DIWSTRT
            | custom_reg::denise::DIWSTOP
            | custom_reg::denise::BPLCON1
            | custom_reg::denise::CLXCON
            | custom_reg::denise::BPLCON2
            | custom_reg::denise::BPLCON3
            | custom_reg::denise::BPL1DAT..=custom_reg::denise::BPL6DAT
            | custom_reg::denise::SPR0POS..=custom_reg::denise::SPR7DATB
            | custom_reg::denise::COLOR00..=custom_reg::denise::COLOR31
            | custom_reg::denise::JOYTEST => {
                self.denise.write_register(offset, val);
            }
            custom_reg::paula::INTENA
            | custom_reg::paula::INTREQ
            | custom_reg::paula::ADKCON
            | custom_reg::paula::DSKDAT
            | custom_reg::paula::DSKLEN
            | custom_reg::paula::DSKSYNC
            | custom_reg::paula::SERDAT..=custom_reg::paula::POTGO
            | custom_reg::paula::AUD0LEN
            | custom_reg::paula::AUD0PER
            | custom_reg::paula::AUD0VOL
            | custom_reg::paula::AUD0DAT
            | custom_reg::paula::AUD1LEN
            | custom_reg::paula::AUD1PER
            | custom_reg::paula::AUD1VOL
            | custom_reg::paula::AUD1DAT
            | custom_reg::paula::AUD2LEN
            | custom_reg::paula::AUD2PER
            | custom_reg::paula::AUD2VOL
            | custom_reg::paula::AUD2DAT
            | custom_reg::paula::AUD3LEN
            | custom_reg::paula::AUD3PER
            | custom_reg::paula::AUD3VOL
            | custom_reg::paula::AUD3DAT => {
                self.paula.write_register(offset, val);
            }
            _ => {
                self.agnus.write_register(offset, val);
            }
        }
    }

    /// Writes an 8-bit byte to custom register space ($DF0000..$DFFFFF).
    ///
    /// Per M68000 bus reality and custom chip architecture, A0 is disconnected from
    /// custom chips and UDS/LDS are not gated; the byte is duplicated on both byte lanes
    /// (D15..D8 and D7..D0) and written as a 16-bit word `(val << 8) | val`.
    #[inline(always)]
    pub fn write_custom_byte(&mut self, addr: u32, val: u8) {
        let offset = (addr & CUSTOM_REG_OFFSET_MASK as u32) as u16;
        let word_val = ((val as u16) << 8) | (val as u16);
        self.write_custom_word(offset, word_val);
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
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            BANK_CUSTOM => {
                let word = self.read_custom_word((addr & CUSTOM_REG_OFFSET_MASK as u32) as u16);
                let byte = if (addr & 1) == 0 {
                    (word >> 8) as u8
                } else {
                    (word & 0xFF) as u8
                };
                BusResult::Ready(byte)
            }
            BANK_CIA => BusResult::Ready(self.read_cia_byte(addr)),
            BANK_RTC => {
                if (RTC_START..=RTC_END).contains(&addr) {
                    BusResult::Ready(self.rtc.read_byte(addr))
                } else {
                    BusResult::Ready(self.mem.unmapped_byte())
                }
            }
            _ => self.mem.read_byte(addr),
        }
    }

    #[inline(always)]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            BANK_CUSTOM => BusResult::Ready(
                self.read_custom_word((addr & CUSTOM_REG_OFFSET_MASK as u32) as u16),
            ),
            BANK_CIA => {
                let b0 = self.read_cia_byte(addr);
                let b1 = self.read_cia_byte(addr.wrapping_add(1));
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            BANK_RTC => {
                let b0 = if (RTC_START..=RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (RTC_START..=RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    self.mem.unmapped_byte()
                };
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            _ => self.mem.read_word(addr),
        }
    }

    #[inline(always)]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            BANK_CUSTOM => {
                self.write_custom_byte(addr, val);
                BusResult::Ready(())
            }
            BANK_CIA => {
                self.write_cia_byte(addr, val);
                BusResult::Ready(())
            }
            BANK_RTC => {
                if (RTC_START..=RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, val);
                }
                BusResult::Ready(())
            }
            _ => self.mem.write_byte(addr, val),
        }
    }

    #[inline(always)]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            BANK_CUSTOM => {
                let offset = (addr & CUSTOM_REG_OFFSET_MASK as u32) as u16;
                self.write_custom_word(offset, val);
                BusResult::Ready(())
            }
            BANK_CIA => {
                let bytes = val.to_be_bytes();
                self.write_cia_byte(addr, bytes[0]);
                self.write_cia_byte(addr.wrapping_add(1), bytes[1]);
                BusResult::Ready(())
            }
            BANK_RTC => {
                let bytes = val.to_be_bytes();
                if (RTC_START..=RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, bytes[0]);
                }
                let next_addr = addr.wrapping_add(1);
                if (RTC_START..=RTC_END).contains(&next_addr) {
                    self.rtc.write_byte(next_addr, bytes[1]);
                }
                BusResult::Ready(())
            }
            _ => self.mem.write_word(addr, val),
        }
    }

    #[inline(always)]
    fn read_word_debug(&self, addr: u32) -> u16 {
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            BANK_CUSTOM => {
                self.read_custom_word_debug((addr & CUSTOM_REG_OFFSET_MASK as u32) as u16)
            }
            BANK_CIA => {
                let b0 = self.read_cia_byte_debug(addr);
                let b1 = self.read_cia_byte_debug(addr.wrapping_add(1));
                u16::from_be_bytes([b0, b1])
            }
            BANK_RTC => {
                let b0 = if (RTC_START..=RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (RTC_START..=RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    self.mem.unmapped_byte()
                };
                u16::from_be_bytes([b0, b1])
            }
            _ => self.mem.read_word_debug(addr),
        }
    }
}
