//! Denise register reads, queued writes and private due-event application.

use super::Denise;
use config::custom_reg::denise as reg;
use config::{MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

const REGISTER_SPR_POS_SELECTOR: u16 = 0;
const REGISTER_SPR_CTL_SELECTOR: u16 = (reg::REGISTER_SPR0CTL - reg::REGISTER_SPR0POS) / 2;
const REGISTER_SPR_DATA_SELECTOR: u16 = (reg::REGISTER_SPR0DATA - reg::REGISTER_SPR0POS) / 2;
const REGISTER_SPR_DATB_SELECTOR: u16 = (reg::REGISTER_SPR0DATB - reg::REGISTER_SPR0POS) / 2;

impl Denise {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        !self.mutations.is_full()
    }

    /// Applies STREQU at accepted selection.
    #[inline]
    pub fn strobe_strequ(&mut self) {
        self.receive_sync_strobe(reg::REGISTER_STREQU);
    }

    /// Applies STRVBL at accepted selection.
    #[inline]
    pub fn strobe_strvbl(&mut self) {
        self.receive_sync_strobe(reg::REGISTER_STRVBL);
    }

    /// Applies STRLONG at accepted selection.
    #[inline]
    pub fn strobe_strlong(&mut self) {
        self.receive_sync_strobe(reg::REGISTER_STRLONG);
    }

    /// Applies STRHOR at accepted selection.
    #[inline]
    pub fn strobe_strhor(&mut self) {
        self.receive_sync_strobe(reg::REGISTER_STRHOR);
    }

    /// Queues BPLCON0; active state changes only when due.
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BPLCON0,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTRT; active state changes only when due.
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_DIWSTRT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTOP; active state changes only when due.
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_DIWSTOP,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprpos family; active state changes only when due.
    #[inline]
    pub fn set_sprpos(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_SPR0POS + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprctl family; active state changes only when due.
    #[inline]
    pub fn set_sprctl(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_SPR0CTL + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPLCON1; active state changes only when due.
    #[inline]
    pub fn set_bplcon1(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BPLCON1,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues CLXCON; active state changes only when due.
    #[inline]
    pub fn set_clxcon(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_CLXCON,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPLCON2; active state changes only when due.
    #[inline]
    pub fn set_bplcon2(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BPLCON2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the bpldat family; active state changes only when due.
    #[inline]
    pub fn set_bpldat(&mut self, index: usize, val: u16) -> bool {
        if index >= config::OCS_BITPLANE_COUNT {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_BPL1DAT + (index as u16) * 2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprdata family; active state changes only when due.
    #[inline]
    pub fn set_sprdata(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_SPR0DATA + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprdatb family; active state changes only when due.
    #[inline]
    pub fn set_sprdatb(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_SPR0DATB + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the color family; active state changes only when due.
    #[inline]
    pub fn set_color(&mut self, index: usize, val: u16) -> bool {
        if index >= Self::COLOR_PALETTE_SIZE {
            return false;
        }
        self.mutations.stage(
            reg::REGISTER_COLOR00 + (index as u16) * 2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues JOYTEST; active state changes only when due.
    #[inline]
    pub fn set_joytest(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_JOYTEST,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Commits a register write directly into active Denise silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & config::MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_BPLCON0 => self.apply_bplcon0(val),
            reg::REGISTER_BPLCON1 => self.apply_bplcon1(val),
            reg::REGISTER_BPLCON2 => self.apply_bplcon2(val),
            reg::REGISTER_DIWSTRT => self.diwstrt = val,
            reg::REGISTER_DIWSTOP => self.diwstop = val,
            reg::REGISTER_CLXCON => self.clxcon = val,

            // Color palette $180..$1BE
            reg::REGISTER_COLOR00
            | reg::REGISTER_COLOR01
            | reg::REGISTER_COLOR02
            | reg::REGISTER_COLOR03
            | reg::REGISTER_COLOR04
            | reg::REGISTER_COLOR05
            | reg::REGISTER_COLOR06
            | reg::REGISTER_COLOR07
            | reg::REGISTER_COLOR08
            | reg::REGISTER_COLOR09
            | reg::REGISTER_COLOR10
            | reg::REGISTER_COLOR11
            | reg::REGISTER_COLOR12
            | reg::REGISTER_COLOR13
            | reg::REGISTER_COLOR14
            | reg::REGISTER_COLOR15
            | reg::REGISTER_COLOR16
            | reg::REGISTER_COLOR17
            | reg::REGISTER_COLOR18
            | reg::REGISTER_COLOR19
            | reg::REGISTER_COLOR20
            | reg::REGISTER_COLOR21
            | reg::REGISTER_COLOR22
            | reg::REGISTER_COLOR23
            | reg::REGISTER_COLOR24
            | reg::REGISTER_COLOR25
            | reg::REGISTER_COLOR26
            | reg::REGISTER_COLOR27
            | reg::REGISTER_COLOR28
            | reg::REGISTER_COLOR29
            | reg::REGISTER_COLOR30
            | reg::REGISTER_COLOR31 => {
                let idx = ((offset - reg::REGISTER_COLOR00) / 2) as usize;
                if idx < Self::COLOR_PALETTE_SIZE {
                    self.color[idx] = val & frame_builder::MASK_RGB444_COLOR;
                }
            }

            // Bitplane parallel data latches
            reg::REGISTER_BPL1DAT
            | reg::REGISTER_BPL2DAT
            | reg::REGISTER_BPL3DAT
            | reg::REGISTER_BPL4DAT
            | reg::REGISTER_BPL5DAT
            | reg::REGISTER_BPL6DAT => {
                let plane = ((offset - reg::REGISTER_BPL1DAT) / 2) as usize;
                if plane < config::OCS_BITPLANE_COUNT {
                    self.apply_bpldat(plane, val);
                }
            }

            // Sprite registers
            reg::REGISTER_SPR0POS
            | reg::REGISTER_SPR0CTL
            | reg::REGISTER_SPR0DATA
            | reg::REGISTER_SPR0DATB
            | reg::REGISTER_SPR1POS
            | reg::REGISTER_SPR1CTL
            | reg::REGISTER_SPR1DATA
            | reg::REGISTER_SPR1DATB
            | reg::REGISTER_SPR2POS
            | reg::REGISTER_SPR2CTL
            | reg::REGISTER_SPR2DATA
            | reg::REGISTER_SPR2DATB
            | reg::REGISTER_SPR3POS
            | reg::REGISTER_SPR3CTL
            | reg::REGISTER_SPR3DATA
            | reg::REGISTER_SPR3DATB
            | reg::REGISTER_SPR4POS
            | reg::REGISTER_SPR4CTL
            | reg::REGISTER_SPR4DATA
            | reg::REGISTER_SPR4DATB
            | reg::REGISTER_SPR5POS
            | reg::REGISTER_SPR5CTL
            | reg::REGISTER_SPR5DATA
            | reg::REGISTER_SPR5DATB
            | reg::REGISTER_SPR6POS
            | reg::REGISTER_SPR6CTL
            | reg::REGISTER_SPR6DATA
            | reg::REGISTER_SPR6DATB
            | reg::REGISTER_SPR7POS
            | reg::REGISTER_SPR7CTL
            | reg::REGISTER_SPR7DATA
            | reg::REGISTER_SPR7DATB => {
                let spr =
                    ((offset - reg::REGISTER_SPR0POS) / config::SPRITE_REGISTER_STRIDE) as usize;
                let sub = ((offset - reg::REGISTER_SPR0POS) % config::SPRITE_REGISTER_STRIDE) / 2;
                if spr < config::SPRITE_COUNT {
                    match sub {
                        REGISTER_SPR_POS_SELECTOR => self.sprites.channels[spr].pos = val,
                        REGISTER_SPR_CTL_SELECTOR => {
                            self.sprites.channels[spr].ctl = val;
                            self.sprites.channels[spr].is_armed = false;
                        }
                        REGISTER_SPR_DATA_SELECTOR => {
                            self.sprites.channels[spr].data_a = val;
                            self.sprites.channels[spr].is_armed = true;
                        }
                        REGISTER_SPR_DATB_SELECTOR => self.sprites.channels[spr].data_b = val,
                        _ => {}
                    }
                }
            }

            reg::REGISTER_STREQU
            | reg::REGISTER_STRVBL
            | reg::REGISTER_STRHOR
            | reg::REGISTER_STRLONG => self.receive_sync_strobe(offset),
            reg::REGISTER_JOYTEST => self.apply_joytest(val),
            _ => {}
        }
    }

    /// Writes a 16-bit word into BPLxDAT ($110 + plane * 2).
    ///
    /// When plane 0 (BPL1DAT) is written, all 6 bpldat latches are transferred into the active shift registers.
    #[inline]
    fn apply_bpldat(&mut self, plane: usize, val: u16) {
        if plane < config::OCS_BITPLANE_COUNT {
            self.bpldat[plane] = val;
            if plane == 0 {
                self.bpldat_pipe = self.bpldat;
                self.shifters = self.bpldat_pipe;
                self.bpl_armed = true;
            }
        }
    }

    /// Action method: sets BPLCON0 and updates active display mode flags
    #[inline]
    fn apply_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
    }

    /// Action method: sets BPLCON1 horizontal scroll offsets
    #[inline]
    fn apply_bplcon1(&mut self, val: u16) {
        self.bplcon1 = val;
    }

    /// Action method: sets BPLCON2 priority flags
    #[inline]
    fn apply_bplcon2(&mut self, val: u16) {
        self.bplcon2 = val;
    }

    /// Reads collision data register (CLXDAT at $DFF00E) and clears it immediately on read
    #[inline]
    pub fn read_clxdat(&mut self) -> u16 {
        let val = self.clxdat;
        self.clxdat = 0;
        val
    }

    /// Reads register with physical read side-effects (e.g. clearing CLXDAT).
    /// Write-only registers return floating open bus $FFFF.
    pub fn read_register(&mut self, offset: u16) -> u16 {
        match offset & config::MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_JOY0DAT => self.joy0dat,
            reg::REGISTER_JOY1DAT => self.joy1dat,
            reg::REGISTER_CLXDAT => self.read_clxdat(),
            _ => config::OPEN_BUS_WORD,
        }
    }

    /// Reads Joystick/Mouse 0 data register (JOY0DAT at $DFF00A)
    #[inline(always)]
    pub fn joy0dat(&self) -> u16 {
        self.joy0dat
    }

    /// Reads Joystick/Mouse 0 data register without side-effects for debugging
    #[inline(always)]
    pub fn joy0dat_debug(&self) -> u16 {
        self.joy0dat
    }

    /// Reads Joystick/Mouse 1 data register (JOY1DAT at $DFF00C)
    #[inline(always)]
    pub fn joy1dat(&self) -> u16 {
        self.joy1dat
    }

    /// Reads Joystick/Mouse 1 data register without side-effects for debugging
    #[inline(always)]
    pub fn joy1dat_debug(&self) -> u16 {
        self.joy1dat
    }

    /// Reads Collision Data register with clear-on-read side-effect (CLXDAT at $DFF00E)
    #[inline(always)]
    pub fn clxdat(&mut self) -> u16 {
        self.read_clxdat()
    }

    /// Reads Collision Data register without clearing for debugging (CLXDAT at $DFF00E)
    #[inline(always)]
    pub fn clxdat_debug(&self) -> u16 {
        self.clxdat
    }
}
