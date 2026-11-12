//! Applies matured Agnus writes to active latches and owned engines.

use super::Agnus;
use config::chip_ram::{MASK_OCS_DMA_ADDRESS, MASK_OCS_DMA_HIGH, MASK_OCS_DMA_LOW};
use config::custom_reg::agnus as reg;
use config::mask::dmacon;
use config::{
    display_window, AUDIO_CHANNEL_COUNT, AUDIO_REGISTER_STRIDE, BITPLANE_POINTER_STRIDE,
    BIT_POINTER_LOW_REGISTER, MASK_CUSTOM_REG_OFFSET, OCS_BITPLANE_COUNT, SPRITE_COUNT,
    SPRITE_POINTER_STRIDE, SPRITE_REGISTER_STRIDE,
};

impl Agnus {
    fn write_dma_pointer_high(pointer: u32, value: u16) -> u32 {
        (pointer & u32::from(MASK_OCS_DMA_LOW)) | (u32::from(value & MASK_OCS_DMA_HIGH) << 16)
    }
    fn write_dma_pointer_low(pointer: u32, value: u16) -> u32 {
        (pointer & (MASK_OCS_DMA_ADDRESS & 0xFFFF_0000)) | u32::from(value & MASK_OCS_DMA_LOW)
    }
    /// Commits a register write directly into active Agnus silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_DMACON => {
                // DMACON SET/CLR logic
                if (val & dmacon::BIT_SET_CLR) != 0 {
                    self.dmacon |= val & dmacon::MASK_WRITE;
                } else {
                    self.dmacon &= !(val & dmacon::MASK_WRITE);
                }
                self.dma.write_dmacon(val);
                let dma_en = self.is_dma_enabled(dmacon::BIT_BLTEN);
                self.blitter.set_dma_enabled(dma_en);
                self.blitter.set_bltpri(self.is_blitter_nasty());
                self.copper
                    .set_dma_enabled(self.is_dma_enabled(dmacon::BIT_COPEN));
            }
            reg::REGISTER_COPCON => self.copper.set_copcon(val),
            reg::REGISTER_COP1LCH => {
                self.copper.cop1lc = Self::write_dma_pointer_high(self.copper.cop1lc, val)
            }
            reg::REGISTER_COP1LCL => {
                self.copper.cop1lc = Self::write_dma_pointer_low(self.copper.cop1lc, val)
            }
            reg::REGISTER_COP2LCH => {
                self.copper.cop2lc = Self::write_dma_pointer_high(self.copper.cop2lc, val)
            }
            reg::REGISTER_COP2LCL => {
                self.copper.cop2lc = Self::write_dma_pointer_low(self.copper.cop2lc, val)
            }
            reg::REGISTER_COPJMP1 => self.strobe_copjmp1(),
            reg::REGISTER_COPJMP2 => self.strobe_copjmp2(),
            reg::REGISTER_DIWSTRT => {
                self.diwstrt = val;
                self.dma.set_vstart(val >> 8);
            }
            reg::REGISTER_DIWSTOP => {
                self.diwstop = val;
                let vstop = (val >> 8)
                    | (if val & display_window::BIT_VSTOP_EXTENSION_SOURCE == 0 {
                        display_window::BIT_VSTOP_EXTENSION
                    } else {
                        0
                    });
                self.dma.set_vstop(vstop);
            }
            reg::REGISTER_DDFSTRT => {
                self.ddfstrt = val & display_window::MASK_DDF_POSITION;
                self.dma
                    .set_ddfstrt(val & display_window::MASK_DDF_POSITION);
            }
            reg::REGISTER_DDFSTOP => {
                self.ddfstop = val & display_window::MASK_DDF_POSITION;
                self.dma
                    .set_ddfstop(val & display_window::MASK_DDF_POSITION);
            }
            reg::REGISTER_BPLCON0 => self.apply_bplcon0(val),
            reg::REGISTER_BPL1MOD => self.bpl1mod = val as i16,
            reg::REGISTER_BPL2MOD => self.bpl2mod = val as i16,

            // Blitter registers
            reg::REGISTER_BLTCON0 => self.blitter.bltcon0 = val,
            reg::REGISTER_BLTCON1 => self.blitter.bltcon1 = val,
            reg::REGISTER_BLTAFWM => self.blitter.bltafwm = val,
            reg::REGISTER_BLTALWM => self.blitter.bltalwm = val,
            reg::REGISTER_BLTCPTH => {
                self.blitter.bltcpt = Self::write_dma_pointer_high(self.blitter.bltcpt, val)
            }
            reg::REGISTER_BLTCPTL => {
                self.blitter.bltcpt = Self::write_dma_pointer_low(self.blitter.bltcpt, val)
            }
            reg::REGISTER_BLTBPTH => {
                self.blitter.bltbpt = Self::write_dma_pointer_high(self.blitter.bltbpt, val)
            }
            reg::REGISTER_BLTBPTL => {
                self.blitter.bltbpt = Self::write_dma_pointer_low(self.blitter.bltbpt, val)
            }
            reg::REGISTER_BLTAPTH => {
                self.blitter.bltapt = Self::write_dma_pointer_high(self.blitter.bltapt, val)
            }
            reg::REGISTER_BLTAPTL => {
                self.blitter.bltapt = Self::write_dma_pointer_low(self.blitter.bltapt, val)
            }
            reg::REGISTER_BLTDPTH => {
                self.blitter.bltdpt = Self::write_dma_pointer_high(self.blitter.bltdpt, val)
            }
            reg::REGISTER_BLTDPTL => {
                self.blitter.bltdpt = Self::write_dma_pointer_low(self.blitter.bltdpt, val)
            }
            reg::REGISTER_BLTSIZE => self.blitter.trigger_blit(val),
            reg::REGISTER_BLTCMOD => self.blitter.bltcmod = val as i16,
            reg::REGISTER_BLTBMOD => self.blitter.bltbmod = val as i16,
            reg::REGISTER_BLTAMOD => self.blitter.bltamod = val as i16,
            reg::REGISTER_BLTDMOD => self.blitter.bltdmod = val as i16,
            reg::REGISTER_BLTCDAT => self.blitter.bltcdat = val,
            reg::REGISTER_BLTBDAT => self.blitter.bltbdat = val,
            reg::REGISTER_BLTADAT => self.blitter.bltadat = val,

            // Bitplane pointers
            reg::REGISTER_BPL1PTH
            | reg::REGISTER_BPL1PTL
            | reg::REGISTER_BPL2PTH
            | reg::REGISTER_BPL2PTL
            | reg::REGISTER_BPL3PTH
            | reg::REGISTER_BPL3PTL
            | reg::REGISTER_BPL4PTH
            | reg::REGISTER_BPL4PTL
            | reg::REGISTER_BPL5PTH
            | reg::REGISTER_BPL5PTL
            | reg::REGISTER_BPL6PTH
            | reg::REGISTER_BPL6PTL => {
                let plane = ((offset - reg::REGISTER_BPL1PTH) / BITPLANE_POINTER_STRIDE) as usize;
                let is_low = (offset & BIT_POINTER_LOW_REGISTER) != 0;
                if plane < OCS_BITPLANE_COUNT {
                    if is_low {
                        self.bplpt[plane] = Self::write_dma_pointer_low(self.bplpt[plane], val);
                    } else {
                        self.bplpt[plane] = Self::write_dma_pointer_high(self.bplpt[plane], val);
                    }
                }
            }

            // Sprite pointers
            reg::REGISTER_SPR0PTH
            | reg::REGISTER_SPR0PTL
            | reg::REGISTER_SPR1PTH
            | reg::REGISTER_SPR1PTL
            | reg::REGISTER_SPR2PTH
            | reg::REGISTER_SPR2PTL
            | reg::REGISTER_SPR3PTH
            | reg::REGISTER_SPR3PTL
            | reg::REGISTER_SPR4PTH
            | reg::REGISTER_SPR4PTL
            | reg::REGISTER_SPR5PTH
            | reg::REGISTER_SPR5PTL
            | reg::REGISTER_SPR6PTH
            | reg::REGISTER_SPR6PTL
            | reg::REGISTER_SPR7PTH
            | reg::REGISTER_SPR7PTL => {
                let sprite = ((offset - reg::REGISTER_SPR0PTH) / SPRITE_POINTER_STRIDE) as usize;
                let is_low = (offset & BIT_POINTER_LOW_REGISTER) != 0;
                if sprite < SPRITE_COUNT {
                    if is_low {
                        self.sprpt[sprite] = Self::write_dma_pointer_low(self.sprpt[sprite], val);
                    } else {
                        self.sprpt[sprite] = Self::write_dma_pointer_high(self.sprpt[sprite], val);
                    }
                }
            }

            // Sprite position and control latches
            reg::REGISTER_SPR0POS
            | reg::REGISTER_SPR0CTL
            | reg::REGISTER_SPR1POS
            | reg::REGISTER_SPR1CTL
            | reg::REGISTER_SPR2POS
            | reg::REGISTER_SPR2CTL
            | reg::REGISTER_SPR3POS
            | reg::REGISTER_SPR3CTL
            | reg::REGISTER_SPR4POS
            | reg::REGISTER_SPR4CTL
            | reg::REGISTER_SPR5POS
            | reg::REGISTER_SPR5CTL
            | reg::REGISTER_SPR6POS
            | reg::REGISTER_SPR6CTL
            | reg::REGISTER_SPR7POS
            | reg::REGISTER_SPR7CTL => {
                let sprite = ((offset - reg::REGISTER_SPR0POS) / SPRITE_REGISTER_STRIDE) as usize;
                if (offset & BIT_POINTER_LOW_REGISTER) == 0 {
                    self.sprpos[sprite] = val;
                } else {
                    self.sprctl[sprite] = val;
                }
            }

            // Floppy Disk DMA pointer
            reg::REGISTER_DSKPTH => {
                self.dskpt = Self::write_dma_pointer_high(self.dskpt, val);
            }
            reg::REGISTER_DSKPTL => {
                self.dskpt = Self::write_dma_pointer_low(self.dskpt, val);
            }

            // Audio DMA location pointers (AUD0LCH/LCL..AUD3LCH/LCL)
            reg::REGISTER_AUD0LCH
            | reg::REGISTER_AUD0LCL
            | reg::REGISTER_AUD1LCH
            | reg::REGISTER_AUD1LCL
            | reg::REGISTER_AUD2LCH
            | reg::REGISTER_AUD2LCL
            | reg::REGISTER_AUD3LCH
            | reg::REGISTER_AUD3LCL => {
                let ch = ((offset - reg::REGISTER_AUD0LCH) / AUDIO_REGISTER_STRIDE) as usize;
                if ch < AUDIO_CHANNEL_COUNT {
                    if (offset & BIT_POINTER_LOW_REGISTER) == 0 {
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
