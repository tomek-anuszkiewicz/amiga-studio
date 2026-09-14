//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use audio::Audio;
use blitter::Blitter;
use cia::{Cia, CiaId};
use copper::Copper;
use denise::Denise;
use dma::DmaScheduler;
use floppy::FloppyController;
use frame_builder::FrameBuilder;
use memory_bus::{AddressBus, BusResult, PhysicalMemory};
use paula::Paula;
use rtc::RtcMsm6242b;
use sprites::Sprites;

/// Zero-cost stack-allocated router implementing `AddressBus` across all subsystems
pub struct MemoryBus<'a> {
    pub mem: &'a mut PhysicalMemory,
    pub agnus: &'a mut Agnus,
    pub denise: &'a mut Denise,
    pub paula: &'a mut Paula,
    pub cia_a: &'a mut Cia,
    pub cia_b: &'a mut Cia,
    pub rtc: &'a mut RtcMsm6242b,
    pub copper: &'a mut Copper,
    pub blitter: &'a mut Blitter,
    pub dma: &'a mut DmaScheduler,
    pub sprites: &'a mut Sprites,
    pub frame_builder: &'a mut FrameBuilder,
    pub audio: &'a mut Audio,
    pub floppy: &'a mut FloppyController,
}

impl<'a> MemoryBus<'a> {
    /// Reads a 16-bit custom register with live read side-effects (e.g. clearing CLXDAT)
    pub fn read_custom_word(&mut self, offset: u16) -> u16 {
        let offset = offset & 0x1FE;
        match offset {
            0x002 | 0x004 | 0x006 => self.agnus.read_register(offset),
            0x00A | 0x00C | 0x00E => self.denise.read_register(offset),
            0x010..=0x01E => self.paula.read_register(offset),
            _ => 0xFFFF,
        }
    }

    /// Reads a 16-bit custom register without side-effects for debugging inspection
    pub fn peek_custom_word(&self, offset: u16) -> u16 {
        let offset = offset & 0x1FE;
        match offset {
            0x002 | 0x004 | 0x006 => self.agnus.read_register(offset),
            0x00A | 0x00C | 0x00E => self.denise.peek_register(offset),
            0x010..=0x01E => self.paula.read_register(offset),
            _ => 0xFFFF,
        }
    }

    /// Reads an 8-bit byte from CIA register space ($BF0000..$BFFFFF)
    pub fn read_cia_byte(&mut self, addr: u32) -> u8 {
        // CIA-B ($BFD000-$BFDF00): Even byte addresses (A0 = 0)
        if (0xBFD000..=0xBFDF00).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_b.read_register(reg);
            }
            return 0xFF;
        }
        // CIA-A ($BFE001-$BFEF01): Odd byte addresses (A0 = 1)
        if (0xBFE001..=0xBFEF01).contains(&addr) {
            if (addr & 1) == 1 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_a.read_register(reg);
            }
            return 0xFF;
        }
        0xFF
    }

    /// Peeks an 8-bit byte from CIA register space without side-effects
    pub fn peek_cia_byte(&self, addr: u32) -> u8 {
        if (0xBFD000..=0xBFDF00).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                return self.cia_b.peek_register(reg);
            }
            return 0xFF;
        }
        if (0xBFE001..=0xBFEF01).contains(&addr) {
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
        if (0xBFD000..=0xBFDF00).contains(&addr) {
            if (addr & 1) == 0 {
                let reg = ((addr >> 8) & 0x0F) as u8;
                if let Some((r, v)) = self.cia_b.write_register(reg, val) {
                    self.dispatch_cia_action(CiaId::B, r, v);
                }
            }
            return;
        }
        // CIA-A ($BFE001-$BFEF01)
        if (0xBFE001..=0xBFEF01).contains(&addr) && (addr & 1) == 1 {
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
        let offset = offset & 0x1FE;
        match offset {
            // Shared / Broadcast: DMACON ($096) -> Agnus and Paula
            0x096 => {
                if let Some((r, v)) = self.agnus.write_register(0x096, val) {
                    self.dispatch_agnus_action(r, v);
                }
                if let Some((r, v)) = self.paula.write_register(0x096, val) {
                    self.dispatch_paula_action(r, v);
                }
            }
            // Shared / Broadcast: BPLCON0 ($100) -> Denise (1 CCK) & Agnus (4 CCK)
            0x100 => {
                if let Some((r, v)) = self.denise.write_register(0x100, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(0x100, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Shared / Broadcast: BPLCON1 ($102) -> Denise & Agnus
            0x102 => {
                if let Some((r, v)) = self.denise.write_register(0x102, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(0x102, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Shared / Broadcast: DIWSTRT ($08E) & DIWSTOP ($090) -> Denise & Agnus
            0x08E | 0x090 => {
                if let Some((r, v)) = self.denise.write_register(offset, val) {
                    self.dispatch_denise_action(r, v);
                }
                if let Some((r, v)) = self.agnus.write_register(offset, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
            // Denise-specific registers (CLXCON, BPLCON2/3, BPLDAT, SPRITES, COLORS)
            0x098 | 0x104 | 0x106 | 0x110..=0x11A | 0x140..=0x17E | 0x180..=0x1BE | 0x036 => {
                if let Some((r, v)) = self.denise.write_register(offset, val) {
                    self.dispatch_denise_action(r, v);
                }
            }
            // Paula-specific registers (INTENA, INTREQ, ADKCON, UART, DSKLEN/SYNC, AUDIO)
            0x09A
            | 0x09C
            | 0x09E
            | 0x018
            | 0x01A
            | 0x024
            | 0x026
            | 0x030..=0x034
            | 0x07E
            | 0x0A0..=0x0DE => {
                if let Some((r, v)) = self.paula.write_register(offset, val) {
                    self.dispatch_paula_action(r, v);
                }
            }
            // Agnus-specific registers (Blitter, Copper, DMA pointers, modulos, DDF)
            _ => {
                if let Some((r, v)) = self.agnus.write_register(offset, val) {
                    self.dispatch_agnus_action(r, v);
                }
            }
        }
    }

    /// Action method dispatch for committed Agnus registers
    pub fn dispatch_agnus_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            0x096 => {
                self.dma.write_dmacon(val);
                let dmaen = (self.agnus.dmacon & 0x0200) != 0;
                self.audio
                    .set_dma_enables((self.agnus.dmacon & 0x000F) as u8, dmaen);
                self.floppy
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0010) != 0);
                self.sprites
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0020) != 0);
                self.blitter
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0040) != 0);
                self.blitter.set_bltpri((self.agnus.dmacon & 0x0400) != 0);
                self.copper
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0080) != 0);
                self.frame_builder
                    .set_dma_enabled(dmaen && (self.agnus.dmacon & 0x0100) != 0);
            }
            0x088 => {
                self.copper.strobe_jump1(self.agnus.cop1lc);
            }
            0x08A => {
                self.copper.strobe_jump2(self.agnus.cop2lc);
            }
            0x080 | 0x082 => {
                self.copper.set_cop1lc(self.agnus.cop1lc);
            }
            0x084 | 0x086 => {
                self.copper.set_cop2lc(self.agnus.cop2lc);
            }
            0x02E => {
                self.copper.set_copcon(val);
            }
            0x058 => {
                self.blitter.sync_pointers(
                    self.agnus.bltapt,
                    self.agnus.bltbpt,
                    self.agnus.bltcpt,
                    self.agnus.bltdpt,
                );
                self.blitter.sync_controls(
                    self.agnus.bltcon0,
                    self.agnus.bltcon1,
                    self.agnus.bltafwm,
                    self.agnus.bltalwm,
                    self.agnus.bltamod,
                    self.agnus.bltbmod,
                    self.agnus.bltcmod,
                    self.agnus.bltdmod,
                );
                self.blitter.trigger_blit(val);
            }
            0x100 => {
                self.denise.set_bplcon0(val);
            }
            0x102 => {
                self.denise.set_bplcon1(val);
            }
            0x08E | 0x090 => {
                self.denise.set_diw(self.agnus.diwstrt, self.agnus.diwstop);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed Paula registers
    pub fn dispatch_paula_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            0x0A4 => self.audio.set_len(0, val),
            0x0A6 => self.audio.set_per(0, val),
            0x0A8 => self.audio.set_vol(0, (val & 0x7F) as u8),
            0x0AA => self.audio.set_dat(0, val),

            0x0B4 => self.audio.set_len(1, val),
            0x0B6 => self.audio.set_per(1, val),
            0x0B8 => self.audio.set_vol(1, (val & 0x7F) as u8),
            0x0BA => self.audio.set_dat(1, val),

            0x0C4 => self.audio.set_len(2, val),
            0x0C6 => self.audio.set_per(2, val),
            0x0C8 => self.audio.set_vol(2, (val & 0x7F) as u8),
            0x0CA => self.audio.set_dat(2, val),

            0x0D4 => self.audio.set_len(3, val),
            0x0D6 => self.audio.set_per(3, val),
            0x0D8 => self.audio.set_vol(3, (val & 0x7F) as u8),
            0x0DA => self.audio.set_dat(3, val),

            0x020 | 0x022 => self.floppy.set_dskpt(self.agnus.dskpt),
            0x024 => self.floppy.set_dsklen(val),
            0x07E => self.floppy.set_dsksyn(val),
            0x09E => self.floppy.set_adkcon(self.paula.adkcon),
            0x096 => {
                let dmaen = (self.agnus.dmacon & 0x0200) != 0;
                self.floppy
                    .set_dma_enabled(dmaen && (self.paula.dma_enables & 0x0010) != 0);
                self.audio
                    .set_dma_enables((self.paula.dma_enables & 0x000F) as u8, dmaen);
            }
            _ => {}
        }
    }

    /// Action method dispatch for committed Denise registers
    pub fn dispatch_denise_action(&mut self, reg: u16, val: u16) {
        match reg & 0x1FE {
            0x100 => self.denise.set_bplcon0(val),
            0x102 => self.denise.set_bplcon1(val),
            0x104 => self.denise.set_bplcon2(val),
            0x180..=0x1BE => {
                let idx = ((reg - 0x180) / 2) as usize;
                self.denise.set_color(idx, val);
            }
            0x08E | 0x090 => {
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
            0xDF => {
                let word = self.read_custom_word((addr & 0x1FE) as u16);
                let byte = if (addr & 1) == 0 {
                    (word >> 8) as u8
                } else {
                    (word & 0xFF) as u8
                };
                BusResult::Ready(byte)
            }
            0xBF => BusResult::Ready(self.read_cia_byte(addr)),
            0xDC => {
                if (0xDC0000..=0xDC003F).contains(&addr) {
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
            0xDF => BusResult::Ready(self.read_custom_word((addr & 0x1FE) as u16)),
            0xBF => {
                let b0 = self.read_cia_byte(addr);
                let b1 = self.read_cia_byte(addr.wrapping_add(1));
                BusResult::Ready(u16::from_be_bytes([b0, b1]))
            }
            0xDC => {
                let b0 = if (0xDC0000..=0xDC003F).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (0xDC0000..=0xDC003F).contains(&next_addr) {
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
            0xDF => {
                let offset = (addr & 0x1FE) as u16;
                let word_val = if (addr & 1) == 0 {
                    (val as u16) << 8
                } else {
                    val as u16
                };
                self.dispatch_custom_write(offset, word_val);
                BusResult::Ready(())
            }
            0xBF => {
                self.write_cia_byte(addr, val);
                BusResult::Ready(())
            }
            0xDC => {
                if (0xDC0000..=0xDC003F).contains(&addr) {
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
            0xDF => {
                let offset = (addr & 0x1FE) as u16;
                self.dispatch_custom_write(offset, val);
                BusResult::Ready(())
            }
            0xBF => {
                let bytes = val.to_be_bytes();
                self.write_cia_byte(addr, bytes[0]);
                self.write_cia_byte(addr.wrapping_add(1), bytes[1]);
                BusResult::Ready(())
            }
            0xDC => {
                let bytes = val.to_be_bytes();
                if (0xDC0000..=0xDC003F).contains(&addr) {
                    self.rtc.write_byte(addr, bytes[0]);
                }
                let next_addr = addr.wrapping_add(1);
                if (0xDC0000..=0xDC003F).contains(&next_addr) {
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
            0xDF => self.peek_custom_word((addr & 0x1FE) as u16),
            0xBF => {
                let b0 = self.peek_cia_byte(addr);
                let b1 = self.peek_cia_byte(addr.wrapping_add(1));
                u16::from_be_bytes([b0, b1])
            }
            0xDC => {
                let b0 = if (0xDC0000..=0xDC003F).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    self.mem.unmapped_byte()
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (0xDC0000..=0xDC003F).contains(&next_addr) {
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
