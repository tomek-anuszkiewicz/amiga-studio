//! Applies matured Agnus writes to active latches and owned engines.

use super::Agnus;
use config::chip_ram::{OCS_DMA_ADDRESS_MASK, OCS_DMA_HIGH_MASK, OCS_DMA_LOW_MASK};
use config::custom_reg::agnus as reg;
use config::mask::dmacon;

impl Agnus {
    fn write_dma_pointer_high(pointer: u32, value: u16) -> u32 {
        (pointer & u32::from(OCS_DMA_LOW_MASK)) | (u32::from(value & OCS_DMA_HIGH_MASK) << 16)
    }
    fn write_dma_pointer_low(pointer: u32, value: u16) -> u32 {
        (pointer & (OCS_DMA_ADDRESS_MASK & 0xFFFF_0000)) | u32::from(value & OCS_DMA_LOW_MASK)
    }
    /// Commits a register write directly into active Agnus silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            reg::DMACON => {
                // DMACON SET/CLR logic
                if (val & dmacon::SET_CLR) != 0 {
                    self.dmacon |= val & 0x7FFF;
                } else {
                    self.dmacon &= !(val & 0x7FFF);
                }
                self.dma.write_dmacon(val);
                let dma_en = self.is_dma_enabled(dmacon::BLTEN);
                self.blitter.set_dma_enabled(dma_en);
                self.blitter.set_bltpri(self.is_blitter_nasty());
                self.copper
                    .set_dma_enabled(self.is_dma_enabled(dmacon::COPEN));
            }
            reg::COPCON => self.copper.set_copcon(val),
            reg::COP1LCH => {
                self.copper.cop1lc = Self::write_dma_pointer_high(self.copper.cop1lc, val)
            }
            reg::COP1LCL => {
                self.copper.cop1lc = Self::write_dma_pointer_low(self.copper.cop1lc, val)
            }
            reg::COP2LCH => {
                self.copper.cop2lc = Self::write_dma_pointer_high(self.copper.cop2lc, val)
            }
            reg::COP2LCL => {
                self.copper.cop2lc = Self::write_dma_pointer_low(self.copper.cop2lc, val)
            }
            reg::COPJMP1 => self.strobe_copjmp1(),
            reg::COPJMP2 => self.strobe_copjmp2(),
            reg::DIWSTRT => {
                self.diwstrt = val;
                self.dma.set_vstart(val >> 8);
            }
            reg::DIWSTOP => {
                self.diwstop = val;
                let vstop = (val >> 8) | (if val & 0x8000 == 0 { 0x100 } else { 0 });
                self.dma.set_vstop(vstop);
            }
            reg::DDFSTRT => {
                self.ddfstrt = val & 0x00FC;
                self.dma.set_ddfstrt(val & 0x00FC);
            }
            reg::DDFSTOP => {
                self.ddfstop = val & 0x00FC;
                self.dma.set_ddfstop(val & 0x00FC);
            }
            reg::BPLCON0 => self.apply_bplcon0(val),
            reg::BPL1MOD => self.bpl1mod = val as i16,
            reg::BPL2MOD => self.bpl2mod = val as i16,

            // Blitter registers
            reg::BLTCON0 => self.blitter.bltcon0 = val,
            reg::BLTCON1 => self.blitter.bltcon1 = val,
            reg::BLTAFWM => self.blitter.bltafwm = val,
            reg::BLTALWM => self.blitter.bltalwm = val,
            reg::BLTCPTH => {
                self.blitter.bltcpt = Self::write_dma_pointer_high(self.blitter.bltcpt, val)
            }
            reg::BLTCPTL => {
                self.blitter.bltcpt = Self::write_dma_pointer_low(self.blitter.bltcpt, val)
            }
            reg::BLTBPTH => {
                self.blitter.bltbpt = Self::write_dma_pointer_high(self.blitter.bltbpt, val)
            }
            reg::BLTBPTL => {
                self.blitter.bltbpt = Self::write_dma_pointer_low(self.blitter.bltbpt, val)
            }
            reg::BLTAPTH => {
                self.blitter.bltapt = Self::write_dma_pointer_high(self.blitter.bltapt, val)
            }
            reg::BLTAPTL => {
                self.blitter.bltapt = Self::write_dma_pointer_low(self.blitter.bltapt, val)
            }
            reg::BLTDPTH => {
                self.blitter.bltdpt = Self::write_dma_pointer_high(self.blitter.bltdpt, val)
            }
            reg::BLTDPTL => {
                self.blitter.bltdpt = Self::write_dma_pointer_low(self.blitter.bltdpt, val)
            }
            reg::BLTSIZE => self.blitter.trigger_blit(val),
            reg::BLTCMOD => self.blitter.bltcmod = val as i16,
            reg::BLTBMOD => self.blitter.bltbmod = val as i16,
            reg::BLTAMOD => self.blitter.bltamod = val as i16,
            reg::BLTDMOD => self.blitter.bltdmod = val as i16,
            reg::BLTCDAT => self.blitter.bltcdat = val,
            reg::BLTBDAT => self.blitter.bltbdat = val,
            reg::BLTADAT => self.blitter.bltadat = val,

            // Bitplane pointers
            reg::BPL1PTH
            | reg::BPL1PTL
            | reg::BPL2PTH
            | reg::BPL2PTL
            | reg::BPL3PTH
            | reg::BPL3PTL
            | reg::BPL4PTH
            | reg::BPL4PTL
            | reg::BPL5PTH
            | reg::BPL5PTL
            | reg::BPL6PTH
            | reg::BPL6PTL => {
                let plane = ((offset - reg::BPL1PTH) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if plane < 6 {
                    if is_low {
                        self.bplpt[plane] = Self::write_dma_pointer_low(self.bplpt[plane], val);
                    } else {
                        self.bplpt[plane] = Self::write_dma_pointer_high(self.bplpt[plane], val);
                    }
                }
            }

            // Sprite pointers
            reg::SPR0PTH
            | reg::SPR0PTL
            | reg::SPR1PTH
            | reg::SPR1PTL
            | reg::SPR2PTH
            | reg::SPR2PTL
            | reg::SPR3PTH
            | reg::SPR3PTL
            | reg::SPR4PTH
            | reg::SPR4PTL
            | reg::SPR5PTH
            | reg::SPR5PTL
            | reg::SPR6PTH
            | reg::SPR6PTL
            | reg::SPR7PTH
            | reg::SPR7PTL => {
                let sprite = ((offset - reg::SPR0PTH) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if sprite < 8 {
                    if is_low {
                        self.sprpt[sprite] = Self::write_dma_pointer_low(self.sprpt[sprite], val);
                    } else {
                        self.sprpt[sprite] = Self::write_dma_pointer_high(self.sprpt[sprite], val);
                    }
                }
            }

            // Sprite position and control latches
            reg::SPR0POS
            | reg::SPR0CTL
            | reg::SPR1POS
            | reg::SPR1CTL
            | reg::SPR2POS
            | reg::SPR2CTL
            | reg::SPR3POS
            | reg::SPR3CTL
            | reg::SPR4POS
            | reg::SPR4CTL
            | reg::SPR5POS
            | reg::SPR5CTL
            | reg::SPR6POS
            | reg::SPR6CTL
            | reg::SPR7POS
            | reg::SPR7CTL => {
                let sprite = ((offset - reg::SPR0POS) / 8) as usize;
                if (offset & 2) == 0 {
                    self.sprpos[sprite] = val;
                } else {
                    self.sprctl[sprite] = val;
                }
            }

            // Floppy Disk DMA pointer
            reg::DSKPTH => {
                self.dskpt = Self::write_dma_pointer_high(self.dskpt, val);
            }
            reg::DSKPTL => {
                self.dskpt = Self::write_dma_pointer_low(self.dskpt, val);
            }

            // Audio DMA location pointers (AUD0LCH/LCL..AUD3LCH/LCL)
            reg::AUD0LCH
            | reg::AUD0LCL
            | reg::AUD1LCH
            | reg::AUD1LCL
            | reg::AUD2LCH
            | reg::AUD2LCL
            | reg::AUD3LCH
            | reg::AUD3LCL => {
                let ch = ((offset - reg::AUD0LCH) / 0x10) as usize;
                if ch < 4 {
                    if (offset & 2) == 0 {
                        self.audlc[ch] = Self::write_dma_pointer_high(self.audlc[ch], val);
                    } else {
                        self.audlc[ch] = Self::write_dma_pointer_low(self.audlc[ch], val);
                    }
                    self.audpt[ch] = self.audlc[ch];
                }
            }

            _ => {}
        }
    }

    /// Action method: sets BPLCON0 and synchronizes Agnus DMA scheduler
    #[inline]
    fn apply_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
        self.dma.set_bplcon0(val);
    }
}
