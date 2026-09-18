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
            custom_reg::DMACONR => self.agnus.read_dmaconr(),
            custom_reg::VPOSR => self.agnus.vposr(),
            custom_reg::VHPOSR => self.agnus.vhposr(),
            custom_reg::JOY0DAT => self.denise.joy0dat,
            custom_reg::JOY1DAT => self.denise.joy1dat,
            custom_reg::CLXDAT => self.denise.read_clxdat(),
            custom_reg::ADKCONR => self.paula.adkcon,
            custom_reg::POT0DAT => self.paula.pot0dat,
            custom_reg::POT1DAT => self.paula.pot1dat,
            custom_reg::POTGOR => self.paula.potgor,
            custom_reg::SERDATR => self.paula.serial_port.serdatr,
            custom_reg::DSKBYTR => self.floppy.read_dskbytr(),
            custom_reg::INTENAR => self.paula.intena,
            custom_reg::INTREQR => self.paula.intreq,
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
            custom_reg::DMACONR => self.agnus.read_dmaconr(),
            custom_reg::VPOSR => self.agnus.vposr(),
            custom_reg::VHPOSR => self.agnus.vhposr(),
            custom_reg::JOY0DAT => self.denise.joy0dat,
            custom_reg::JOY1DAT => self.denise.joy1dat,
            custom_reg::CLXDAT => self.denise.clxdat,
            custom_reg::ADKCONR => self.paula.adkcon,
            custom_reg::POT0DAT => self.paula.pot0dat,
            custom_reg::POT1DAT => self.paula.pot1dat,
            custom_reg::POTGOR => self.paula.potgor,
            custom_reg::SERDATR => self.paula.serial_port.serdatr,
            custom_reg::DSKBYTR => self.floppy.peek_dskbytr(),
            custom_reg::INTENAR => self.paula.intena,
            custom_reg::INTREQR => self.paula.intreq,
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
                return self.cia_b.peek_register(reg);
            }
            return 0xFF;
        }
        if (CIA_A_START..=CIA_A_END).contains(&addr) {
            if (addr & 1) == 1 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_a.peek_register(reg);
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
                if let Some((r, v)) = self.cia_b.write_register(reg, val) {
                    self.dispatch_cia_action(CiaId::B, r, v);
                }
            }
            return;
        }
        // CIA-A ($BFE001-$BFEF01)
        if (CIA_A_START..=CIA_A_END).contains(&addr) && (addr & 1) == 1 {
            let reg = ((addr >> 8) & 0x0F) as u8;
            if let Some((r, v)) = self.cia_a.write_register(reg, val) {
                self.dispatch_cia_action(CiaId::A, r, v);
            }
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

    /// Dispatches a custom register bus write to the target chip(s) with physical propagation delay.
    pub fn dispatch_custom_write(&mut self, offset: u16, val: u16) {
        let offset = offset & CUSTOM_REG_OFFSET_MASK;
        match offset {
            // Master DMA control: staged in Agnus, broadcasts to all chips on commit
            custom_reg::DMACON => {
                if let Some((r, v)) = self.agnus.write_register(custom_reg::DMACON, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Shared / Broadcast: BPLCON0 ($100) -> Denise (1 CCK) & Agnus (4 CCK)
            custom_reg::BPLCON0 => {
                if let Some((r, v)) = self.denise.write_register(custom_reg::BPLCON0, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(custom_reg::BPLCON0, val) {
                    self.dispatch_agnus_action(r, v);
                }
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
                if let Some((r, v)) = self.denise.write_register(offset, val) {
                    self.dispatch_denise_action(r, v);
                }
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
                if let Some((r, v)) = self.paula.write_register(offset, val) {
                    self.dispatch_paula_action(r, v);
                }
            }
            // Agnus-specific registers (Blitter, Copper, DMA pointers, modulos, DDF, AUDxLC)
            _ => {
                if let Some((r, v)) = self.agnus.write_register(offset, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
        }
    }

    /// Action method dispatch for committed Agnus registers
    pub fn dispatch_agnus_action(&mut self, reg: u16, val: u16) {
        match reg & CUSTOM_REG_OFFSET_MASK {
            custom_reg::DMACON => {
                self.agnus.dma.write_dmacon(val);
                // Broadcast to Paula's dma_enables
                if (val & dmacon::SET_CLR) != 0 {
                    self.paula.dma_enables |= val & (dmacon::AUD_ALL | dmacon::DSKEN);
                } else {
                    self.paula.dma_enables &= !(val & (dmacon::AUD_ALL | dmacon::DSKEN));
                }
                let dmaen = (self.agnus.dmacon & dmacon::DMAEN) != 0;
                self.paula.dma_master = dmaen;
                self.paula
                    .audio
                    .set_dma_enables((self.agnus.dmacon & dmacon::AUD_ALL) as u8, dmaen);
                self.floppy
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & dmacon::DSKEN) != 0);
                self.denise
                    .sprites
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & dmacon::SPREN) != 0);
                self.denise
                    .frame_builder
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & dmacon::BPLEN) != 0);
            }
            custom_reg::BPLCON0 => {
                self.agnus.set_bplcon0(val);
            }
            custom_reg::COPJMP1 => {
                self.agnus.strobe_copjmp1();
            }
            custom_reg::COPJMP2 => {
                self.agnus.strobe_copjmp2();
            }
            custom_reg::BLTSIZE => {
                self.agnus.blitter.trigger_blit(val);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed Paula registers
    pub fn dispatch_paula_action(&mut self, reg: u16, val: u16) {
        match reg & CUSTOM_REG_OFFSET_MASK {
            custom_reg::DSKLEN => self.floppy.set_dsklen(val),
            custom_reg::DSKSYNC => self.floppy.set_dsksyn(val),
            custom_reg::ADKCON => self.floppy.set_adkcon(self.paula.adkcon),
            _ => {}
        }
    }

    /// Action method dispatch for committed Denise registers
    pub fn dispatch_denise_action(&mut self, reg: u16, val: u16) {
        match reg & CUSTOM_REG_OFFSET_MASK {
            custom_reg::BPLCON0 => self.denise.set_bplcon0(val),
            custom_reg::BPLCON1 => self.denise.set_bplcon1(val),
            custom_reg::BPLCON2 => self.denise.set_bplcon2(val),
            custom_reg::COLOR00..=custom_reg::COLOR31 => {
                let idx = ((reg - custom_reg::COLOR00) / 2) as usize;
                self.denise.set_color(idx, val);
            }
            custom_reg::DIWSTRT | custom_reg::DIWSTOP => {
                self.denise
                    .set_diw(self.denise.diwstrt, self.denise.diwstop);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed CIA registers
    pub fn dispatch_cia_action(&mut self, id: CiaId, reg: u8, val: u8) {
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
                let offset = (addr & CUSTOM_REG_OFFSET_MASK as u32) as u16;
                let word_val = if (addr & 1) == 0 {
                    (val as u16) << 8
                } else {
                    val as u16
                };
                self.dispatch_custom_write(offset, word_val);
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
                self.dispatch_custom_write(offset, val);
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
