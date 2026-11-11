//! Denise register reads, queued writes and private due-event application.

use super::Denise;
use config::custom_reg::denise as reg;
use config::{stage_mutation, MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Denise {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        self.mutations.iter().any(Option::is_none)
    }

    /// Applies STREQU at accepted selection.
    #[inline]
    pub fn strobe_strequ(&mut self) {
        self.sync.receive_strobe(reg::STREQU);
    }

    /// Applies STRVBL at accepted selection.
    #[inline]
    pub fn strobe_strvbl(&mut self) {
        self.sync.receive_strobe(reg::STRVBL);
    }

    /// Applies STRLONG at accepted selection.
    #[inline]
    pub fn strobe_strlong(&mut self) {
        self.sync.receive_strobe(reg::STRLONG);
    }

    /// Applies STRHOR at accepted selection.
    #[inline]
    pub fn strobe_strhor(&mut self) {
        self.sync.receive_strobe(reg::STRHOR);
    }

    /// Queues BPLCON0; active state changes only when due.
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON0,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTRT; active state changes only when due.
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTRT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTOP; active state changes only when due.
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTOP,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprpos family; active state changes only when due.
    #[inline]
    pub fn set_sprpos(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0POS + (index as u16) * 8,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprctl family; active state changes only when due.
    #[inline]
    pub fn set_sprctl(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0CTL + (index as u16) * 8,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPLCON1; active state changes only when due.
    #[inline]
    pub fn set_bplcon1(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON1,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues CLXCON; active state changes only when due.
    #[inline]
    pub fn set_clxcon(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::CLXCON,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPLCON2; active state changes only when due.
    #[inline]
    pub fn set_bplcon2(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the bpldat family; active state changes only when due.
    #[inline]
    pub fn set_bpldat(&mut self, index: usize, val: u16) -> bool {
        if index >= 6 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::BPL1DAT + (index as u16) * 2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprdata family; active state changes only when due.
    #[inline]
    pub fn set_sprdata(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0DATA + (index as u16) * 8,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprdatb family; active state changes only when due.
    #[inline]
    pub fn set_sprdatb(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0DATB + (index as u16) * 8,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the color family; active state changes only when due.
    #[inline]
    pub fn set_color(&mut self, index: usize, val: u16) -> bool {
        if index >= 32 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::COLOR00 + (index as u16) * 2,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues JOYTEST; active state changes only when due.
    #[inline]
    pub fn set_joytest(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::JOYTEST,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Commits a register write directly into active Denise silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            reg::BPLCON0 => self.apply_bplcon0(val),
            reg::BPLCON1 => self.apply_bplcon1(val),
            reg::BPLCON2 => self.apply_bplcon2(val),
            reg::DIWSTRT => self.diwstrt = val,
            reg::DIWSTOP => self.diwstop = val,
            reg::CLXCON => self.clxcon = val,

            // Color palette $180..$1BE
            reg::COLOR00
            | reg::COLOR01
            | reg::COLOR02
            | reg::COLOR03
            | reg::COLOR04
            | reg::COLOR05
            | reg::COLOR06
            | reg::COLOR07
            | reg::COLOR08
            | reg::COLOR09
            | reg::COLOR10
            | reg::COLOR11
            | reg::COLOR12
            | reg::COLOR13
            | reg::COLOR14
            | reg::COLOR15
            | reg::COLOR16
            | reg::COLOR17
            | reg::COLOR18
            | reg::COLOR19
            | reg::COLOR20
            | reg::COLOR21
            | reg::COLOR22
            | reg::COLOR23
            | reg::COLOR24
            | reg::COLOR25
            | reg::COLOR26
            | reg::COLOR27
            | reg::COLOR28
            | reg::COLOR29
            | reg::COLOR30
            | reg::COLOR31 => {
                let idx = ((offset - reg::COLOR00) / 2) as usize;
                if idx < Self::COLOR_PALETTE_SIZE {
                    self.color[idx] = val & 0x0FFF;
                }
            }

            // Bitplane parallel data latches
            reg::BPL1DAT
            | reg::BPL2DAT
            | reg::BPL3DAT
            | reg::BPL4DAT
            | reg::BPL5DAT
            | reg::BPL6DAT => {
                let plane = ((offset - reg::BPL1DAT) / 2) as usize;
                if plane < 6 {
                    self.apply_bpldat(plane, val);
                }
            }

            // Sprite registers
            reg::SPR0POS
            | reg::SPR0CTL
            | reg::SPR0DATA
            | reg::SPR0DATB
            | reg::SPR1POS
            | reg::SPR1CTL
            | reg::SPR1DATA
            | reg::SPR1DATB
            | reg::SPR2POS
            | reg::SPR2CTL
            | reg::SPR2DATA
            | reg::SPR2DATB
            | reg::SPR3POS
            | reg::SPR3CTL
            | reg::SPR3DATA
            | reg::SPR3DATB
            | reg::SPR4POS
            | reg::SPR4CTL
            | reg::SPR4DATA
            | reg::SPR4DATB
            | reg::SPR5POS
            | reg::SPR5CTL
            | reg::SPR5DATA
            | reg::SPR5DATB
            | reg::SPR6POS
            | reg::SPR6CTL
            | reg::SPR6DATA
            | reg::SPR6DATB
            | reg::SPR7POS
            | reg::SPR7CTL
            | reg::SPR7DATA
            | reg::SPR7DATB => {
                let spr = ((offset - reg::SPR0POS) / 8) as usize;
                let sub = ((offset - reg::SPR0POS) % 8) / 2;
                if spr < 8 {
                    match sub {
                        0 => self.sprites.channels[spr].pos = val,
                        1 => {
                            self.sprites.channels[spr].ctl = val;
                            self.sprites.channels[spr].is_armed = false;
                        }
                        2 => {
                            self.sprites.channels[spr].data_a = val;
                            self.sprites.channels[spr].is_armed = true;
                        }
                        3 => self.sprites.channels[spr].data_b = val,
                        _ => {}
                    }
                }
            }

            reg::STREQU | reg::STRVBL | reg::STRHOR | reg::STRLONG => {
                self.sync.receive_strobe(offset)
            }
            reg::JOYTEST => {}
            _ => {}
        }
    }

    /// Writes a 16-bit word into BPLxDAT ($110 + plane * 2).
    ///
    /// When plane 0 (BPL1DAT) is written, all 6 bpldat latches are transferred into the active shift registers.
    #[inline]
    fn apply_bpldat(&mut self, plane: usize, val: u16) {
        if plane < 6 {
            self.bpldat[plane] = val;
            if plane == 0 {
                self.bpldat_pipe = self.bpldat;
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
        match offset & 0x1FE {
            reg::JOY0DAT => self.joy0dat,
            reg::JOY1DAT => self.joy1dat,
            reg::CLXDAT => self.read_clxdat(),
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
