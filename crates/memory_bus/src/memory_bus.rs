//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::{Cia, CiaId};
use config::{cia_reg, custom_reg};
use denise::Denise;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
use rtc::RtcMsm6242b;

pub use physical_memory;

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
    /// Physical memory map constants decoded by Gary logic
    pub const BANK_CIA: u8 = 0xBF;
    pub const BANK_RTC: u8 = 0xDC;
    pub const BANK_CUSTOM: u8 = 0xDF;

    pub const RTC_START: u32 = 0xDC0000;
    pub const RTC_END: u32 = 0xDC003F;

    pub const CIA_REG_ADDRESS_MASK: u32 = 0xFFFF01;
    pub const CUSTOM_REG_OFFSET_MASK: u16 = 0x01FE;

    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT, DSKBYTR)
    #[inline]
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & Self::CUSTOM_REG_OFFSET_MASK;
        match offset {
            // Early DMA reads are not ordinary CPU-readable registers.
            custom_reg::agnus::BLTDDAT | custom_reg::paula::DSKDATR => 0xFFFF,
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
            _ => 0xFFFF,
        }
    }

    fn decode_cia_register(addr: u32) -> Option<(CiaId, u32)> {
        // A7..A1 are not wired to the CIA register selector.
        let canonical_addr = addr & Self::CIA_REG_ADDRESS_MASK;
        let (chip, reg) = match canonical_addr {
            cia_reg::cia_a::PRA => (CiaId::A, cia_reg::cia_a::PRA),
            cia_reg::cia_a::PRB => (CiaId::A, cia_reg::cia_a::PRB),
            cia_reg::cia_a::DDRA => (CiaId::A, cia_reg::cia_a::DDRA),
            cia_reg::cia_a::DDRB => (CiaId::A, cia_reg::cia_a::DDRB),
            cia_reg::cia_a::TALO => (CiaId::A, cia_reg::cia_a::TALO),
            cia_reg::cia_a::TAHI => (CiaId::A, cia_reg::cia_a::TAHI),
            cia_reg::cia_a::TBLO => (CiaId::A, cia_reg::cia_a::TBLO),
            cia_reg::cia_a::TBHI => (CiaId::A, cia_reg::cia_a::TBHI),
            cia_reg::cia_a::TODLO => (CiaId::A, cia_reg::cia_a::TODLO),
            cia_reg::cia_a::TODMID => (CiaId::A, cia_reg::cia_a::TODMID),
            cia_reg::cia_a::TODHI => (CiaId::A, cia_reg::cia_a::TODHI),
            cia_reg::cia_a::SDR => (CiaId::A, cia_reg::cia_a::SDR),
            cia_reg::cia_a::ICR => (CiaId::A, cia_reg::cia_a::ICR),
            cia_reg::cia_a::CRA => (CiaId::A, cia_reg::cia_a::CRA),
            cia_reg::cia_a::CRB => (CiaId::A, cia_reg::cia_a::CRB),
            cia_reg::cia_b::PRA => (CiaId::B, cia_reg::cia_b::PRA),
            cia_reg::cia_b::PRB => (CiaId::B, cia_reg::cia_b::PRB),
            cia_reg::cia_b::DDRA => (CiaId::B, cia_reg::cia_b::DDRA),
            cia_reg::cia_b::DDRB => (CiaId::B, cia_reg::cia_b::DDRB),
            cia_reg::cia_b::TALO => (CiaId::B, cia_reg::cia_b::TALO),
            cia_reg::cia_b::TAHI => (CiaId::B, cia_reg::cia_b::TAHI),
            cia_reg::cia_b::TBLO => (CiaId::B, cia_reg::cia_b::TBLO),
            cia_reg::cia_b::TBHI => (CiaId::B, cia_reg::cia_b::TBHI),
            cia_reg::cia_b::TODLO => (CiaId::B, cia_reg::cia_b::TODLO),
            cia_reg::cia_b::TODMID => (CiaId::B, cia_reg::cia_b::TODMID),
            cia_reg::cia_b::TODHI => (CiaId::B, cia_reg::cia_b::TODHI),
            cia_reg::cia_b::SDR => (CiaId::B, cia_reg::cia_b::SDR),
            cia_reg::cia_b::ICR => (CiaId::B, cia_reg::cia_b::ICR),
            cia_reg::cia_b::CRA => (CiaId::B, cia_reg::cia_b::CRA),
            cia_reg::cia_b::CRB => (CiaId::B, cia_reg::cia_b::CRB),
            _ => return None,
        };
        Some((chip, reg))
    }

    /// Reads an 8-bit byte from CIA register space ($BF0000..$BFFFFF)
    fn read_cia_byte(&mut self, addr: u32) -> u8 {
        match Self::decode_cia_register(addr) {
            Some((CiaId::A, reg)) => self.cia_a.read_register(reg),
            Some((CiaId::B, reg)) => self.cia_b.read_register(reg),
            None => 0xFF,
        }
    }

    /// Reads an 8-bit byte from CIA register space without side-effects for debugging inspection
    pub fn read_cia_byte_debug(&self, addr: u32) -> u8 {
        match Self::decode_cia_register(addr) {
            Some((CiaId::A, reg)) => self.cia_a.read_register_debug(reg),
            Some((CiaId::B, reg)) => self.cia_b.read_register_debug(reg),
            None => 0xFF,
        }
    }

    /// Writes an 8-bit byte to CIA register space ($BF0000..$BFFFFF)
    fn write_cia_byte(&mut self, addr: u32, val: u8) {
        match Self::decode_cia_register(addr) {
            Some((CiaId::B, reg)) => {
                self.cia_b.write_register(reg, val);
            }
            Some((CiaId::A, reg)) => {
                self.cia_a.write_register(reg, val);
                // CIA-A bit 0 of Port A ($BFE001) controls the low-memory overlay (_OVL).
                if reg == cia_reg::cia_a::PRA {
                    if (val & 0x01) == 0 {
                        self.mem.map_kickstart_to_low_memory();
                    } else {
                        self.mem.map_chip_ram_to_low_memory();
                    }
                }
            }
            None => {}
        }
    }

    /// Writes a 16-bit word to custom register space with physical propagation delay.
    #[inline]
    pub fn write_custom_word(&mut self, offset: u16, val: u16) {
        let offset = offset & Self::CUSTOM_REG_OFFSET_MASK;
        match offset {
            custom_reg::denise::STREQU
            | custom_reg::denise::STRVBL
            | custom_reg::denise::STRLONG => self.denise.receive_sync_strobe(offset),
            custom_reg::denise::STRHOR => {
                self.denise.receive_sync_strobe(offset);
                self.paula.receive_horizontal_sync_strobe();
            }
            custom_reg::agnus::DMACON => {
                self.agnus.write_register(custom_reg::agnus::DMACON, val);
                self.paula.write_register(custom_reg::paula::DMACON, val);
            }
            custom_reg::denise::BPLCON0 => {
                self.denise.write_register(custom_reg::denise::BPLCON0, val);
                self.agnus.write_register(custom_reg::agnus::BPLCON0, val);
            }
            custom_reg::denise::SPR0POS
            | custom_reg::denise::SPR0CTL
            | custom_reg::denise::SPR1POS
            | custom_reg::denise::SPR1CTL
            | custom_reg::denise::SPR2POS
            | custom_reg::denise::SPR2CTL
            | custom_reg::denise::SPR3POS
            | custom_reg::denise::SPR3CTL
            | custom_reg::denise::SPR4POS
            | custom_reg::denise::SPR4CTL
            | custom_reg::denise::SPR5POS
            | custom_reg::denise::SPR5CTL
            | custom_reg::denise::SPR6POS
            | custom_reg::denise::SPR6CTL
            | custom_reg::denise::SPR7POS
            | custom_reg::denise::SPR7CTL => {
                self.denise.write_register(offset, val);
                self.agnus.write_register(offset, val);
            }
            custom_reg::denise::DIWSTRT
            | custom_reg::denise::DIWSTOP
            | custom_reg::denise::BPLCON1
            | custom_reg::denise::CLXCON
            | custom_reg::denise::BPLCON2
            | custom_reg::denise::BPL1DAT
            | custom_reg::denise::BPL2DAT
            | custom_reg::denise::BPL3DAT
            | custom_reg::denise::BPL4DAT
            | custom_reg::denise::BPL5DAT
            | custom_reg::denise::BPL6DAT
            | custom_reg::denise::SPR0DATA
            | custom_reg::denise::SPR0DATB
            | custom_reg::denise::SPR1DATA
            | custom_reg::denise::SPR1DATB
            | custom_reg::denise::SPR2DATA
            | custom_reg::denise::SPR2DATB
            | custom_reg::denise::SPR3DATA
            | custom_reg::denise::SPR3DATB
            | custom_reg::denise::SPR4DATA
            | custom_reg::denise::SPR4DATB
            | custom_reg::denise::SPR5DATA
            | custom_reg::denise::SPR5DATB
            | custom_reg::denise::SPR6DATA
            | custom_reg::denise::SPR6DATB
            | custom_reg::denise::SPR7DATA
            | custom_reg::denise::SPR7DATB
            | custom_reg::denise::COLOR00
            | custom_reg::denise::COLOR01
            | custom_reg::denise::COLOR02
            | custom_reg::denise::COLOR03
            | custom_reg::denise::COLOR04
            | custom_reg::denise::COLOR05
            | custom_reg::denise::COLOR06
            | custom_reg::denise::COLOR07
            | custom_reg::denise::COLOR08
            | custom_reg::denise::COLOR09
            | custom_reg::denise::COLOR10
            | custom_reg::denise::COLOR11
            | custom_reg::denise::COLOR12
            | custom_reg::denise::COLOR13
            | custom_reg::denise::COLOR14
            | custom_reg::denise::COLOR15
            | custom_reg::denise::COLOR16
            | custom_reg::denise::COLOR17
            | custom_reg::denise::COLOR18
            | custom_reg::denise::COLOR19
            | custom_reg::denise::COLOR20
            | custom_reg::denise::COLOR21
            | custom_reg::denise::COLOR22
            | custom_reg::denise::COLOR23
            | custom_reg::denise::COLOR24
            | custom_reg::denise::COLOR25
            | custom_reg::denise::COLOR26
            | custom_reg::denise::COLOR27
            | custom_reg::denise::COLOR28
            | custom_reg::denise::COLOR29
            | custom_reg::denise::COLOR30
            | custom_reg::denise::COLOR31
            | custom_reg::denise::JOYTEST => {
                self.denise.write_register(offset, val);
            }
            custom_reg::paula::INTENA
            | custom_reg::paula::INTREQ
            | custom_reg::paula::ADKCON
            | custom_reg::paula::DSKDAT
            | custom_reg::paula::DSKLEN
            | custom_reg::paula::DSKSYNC
            | custom_reg::paula::SERDAT
            | custom_reg::paula::SERPER
            | custom_reg::paula::POTGO
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
        let offset = (addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16;
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
            Self::BANK_CIA => BusResult::Ready(self.read_cia_byte(addr)),
            Self::BANK_RTC => {
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
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
            Self::BANK_CUSTOM => BusResult::Ready(
                self.read_custom_word((addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16),
            ),
            Self::BANK_CIA => {
                let b0 = self.read_cia_byte(addr);
                let b1 = self.read_cia_byte(addr.wrapping_add(1));
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            Self::BANK_RTC => {
                let b0 = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
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
            Self::BANK_CUSTOM => {
                self.write_custom_byte(addr, val);
                BusResult::Ready(())
            }
            Self::BANK_CIA => {
                self.write_cia_byte(addr, val);
                BusResult::Ready(())
            }
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
        let addr = addr & 0x00FF_FFFF;
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
        let addr = addr & 0x00FF_FFFF;
        let bank = (addr >> 16) as u8;
        match bank {
            Self::BANK_CUSTOM => {
                let offset = (addr & Self::CUSTOM_REG_OFFSET_MASK as u32) as u16;
                self.write_custom_word(offset, val);
                BusResult::Ready(())
            }
            Self::BANK_CIA => {
                let bytes = val.to_be_bytes();
                self.write_cia_byte(addr, bytes[0]);
                self.write_cia_byte(addr.wrapping_add(1), bytes[1]);
                BusResult::Ready(())
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
        let addr = addr & 0x00FF_FFFF;
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
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
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
