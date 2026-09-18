//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::{Cia, CiaId};
use config::custom_reg;
use config::mask::dmacon;
use denise::Denise;
use floppy::FloppyController;
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
    pub floppy: &'a mut FloppyController,
}

impl<'a> MemoryBus<'a> {
    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT, DSKBYTR)
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & CUSTOM_REG_OFFSET_MASK;
        match offset {
            custom_reg::DMACONR => self.agnus.dmaconr(),
            custom_reg::VPOSR => self.agnus.vposr(),
            custom_reg::VHPOSR => self.agnus.vhposr(),
            custom_reg::JOY0DAT => self.denise.joy0dat(),
            custom_reg::JOY1DAT => self.denise.joy1dat(),
            custom_reg::CLXDAT => self.denise.clxdat(),
            custom_reg::ADKCONR => self.paula.adkconr(),
            custom_reg::POT0DAT => self.paula.pot0dat(),
            custom_reg::POT1DAT => self.paula.pot1dat(),
            custom_reg::POTGOR => self.paula.potgor(),
            custom_reg::SERDATR => self.paula.serdatr(),
            custom_reg::DSKBYTR => self.read_dskbytr(),
            custom_reg::INTENAR => self.paula.intenar(),
            custom_reg::INTREQR => self.paula.intreqr(),
            custom_reg::COPJMP1 => {
                self.agnus.strobe_copjmp1();
                0xFFFF
            }
            custom_reg::COPJMP2 => {
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
            custom_reg::DMACONR => self.agnus.dmaconr_debug(),
            custom_reg::VPOSR => self.agnus.vposr_debug(),
            custom_reg::VHPOSR => self.agnus.vhposr_debug(),
            custom_reg::JOY0DAT => self.denise.joy0dat_debug(),
            custom_reg::JOY1DAT => self.denise.joy1dat_debug(),
            custom_reg::CLXDAT => self.denise.clxdat_debug(),
            custom_reg::ADKCONR => self.paula.adkconr_debug(),
            custom_reg::POT0DAT => self.paula.pot0dat_debug(),
            custom_reg::POT1DAT => self.paula.pot1dat_debug(),
            custom_reg::POTGOR => self.paula.potgor_debug(),
            custom_reg::SERDATR => self.paula.serdatr_debug(),
            custom_reg::DSKBYTR => self.read_dskbytr_debug(),
            custom_reg::INTENAR => self.paula.intenar_debug(),
            custom_reg::INTREQR => self.paula.intreqr_debug(),
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
                self.write_cia(CiaId::B, reg, val);
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
            // Shared: DMACON ($096) -> Agnus and Paula (both decode master DMA lines on the bus)
            custom_reg::DMACON => {
                self.agnus.write_register(custom_reg::DMACON, val);
                self.paula.write_register(custom_reg::DMACON, val);
            }
            // Shared: BPLCON0 ($100) -> Denise (1 CCK) & Agnus (4 CCK)
            custom_reg::BPLCON0 => {
                self.denise.write_register(custom_reg::BPLCON0, val);
                self.agnus.write_register(custom_reg::BPLCON0, val);
            }
            // Denise-specific registers (DIW, CLXCON, BPLCON1/2/3, BPLDAT, SPRITES, COLORS, JOYTEST)
            custom_reg::DIWSTRT
            | custom_reg::DIWSTOP
            | custom_reg::BPLCON1
            | custom_reg::CLXCON
            | custom_reg::BPLCON2
            | custom_reg::BPLCON3
            | custom_reg::BPL1DAT..=custom_reg::BPL6DAT
            | custom_reg::SPR0POS..=custom_reg::SPR7DATB
            | custom_reg::COLOR00..=custom_reg::COLOR31
            | custom_reg::JOYTEST => {
                self.denise.write_register(offset, val);
            }
            // Paula-specific registers (INTENA, INTREQ, ADKCON, UART, DSKLEN/SYNC, AUDIO length/period/volume/data)
            custom_reg::INTENA
            | custom_reg::INTREQ
            | custom_reg::ADKCON
            | custom_reg::DSKDAT
            | custom_reg::DSKLEN
            | custom_reg::DSKSYNC
            | custom_reg::SERDAT..=custom_reg::POTGO
            | custom_reg::AUD0LEN
            | custom_reg::AUD0PER
            | custom_reg::AUD0VOL
            | custom_reg::AUD0DAT
            | custom_reg::AUD1LEN
            | custom_reg::AUD1PER
            | custom_reg::AUD1VOL
            | custom_reg::AUD1DAT
            | custom_reg::AUD2LEN
            | custom_reg::AUD2PER
            | custom_reg::AUD2VOL
            | custom_reg::AUD2DAT
            | custom_reg::AUD3LEN
            | custom_reg::AUD3PER
            | custom_reg::AUD3VOL
            | custom_reg::AUD3DAT => {
                self.paula.write_register(offset, val);
            }
            // Agnus-specific registers (DMACON, Blitter, Copper, DMA pointers, modulos, DDF, AUDxLC)
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

    /// Reads composite live DSKBYTR status from Floppy and Paula
    #[inline(always)]
    pub fn read_dskbytr(&mut self) -> u16 {
        self.paula.assemble_dskbytr(self.floppy.dskbytr())
    }

    /// Reads composite DSKBYTR status without side-effects for debugging
    #[inline(always)]
    pub fn read_dskbytr_debug(&self) -> u16 {
        self.paula.assemble_dskbytr(self.floppy.dskbytr_debug())
    }

    /// Synchronizes Denise display pipeline DMA enables from Agnus master DMACON state
    #[inline]
    pub fn sync_dmacon(&mut self) {
        let dmacon = self.agnus.dmacon;
        let dmaen = (dmacon & dmacon::DMAEN) != 0;
        self.denise
            .sprites
            .set_dma_enabled(dmaen && (dmacon & dmacon::SPREN) != 0);
        self.denise
            .frame_builder
            .set_dma_enabled(dmaen && (dmacon & dmacon::BPLEN) != 0);
    }

    /// Propagates committed CIA register mutations across the motherboard
    pub fn write_cia(&mut self, id: CiaId, reg: u8, val: u8) {
        if id == CiaId::B && (reg & 0x0F) == 0x1 {
            self.floppy.handle_ciab_port_b_write(val);
        }
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
