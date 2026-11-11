//! Queued Denise register setters and private due-event application.

use super::Denise;
use config::custom_reg::denise as reg;
use config::{stage_mutation, MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Denise {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        self.mutations.iter().any(Option::is_none)
    }

    /// Queues STREQU; active state changes only when due.
    #[inline]
    pub fn set_strequ(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::STREQU,
            0,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::OverwritePending,
        )
    }

    /// Queues STRVBL; active state changes only when due.
    #[inline]
    pub fn set_strvbl(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::STRVBL,
            0,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::OverwritePending,
        )
    }

    /// Queues STRLONG; active state changes only when due.
    #[inline]
    pub fn set_strlong(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::STRLONG,
            0,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::OverwritePending,
        )
    }

    /// Queues STRHOR; active state changes only when due.
    #[inline]
    pub fn set_strhor(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::STRHOR,
            0,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPLCON0; active state changes only when due.
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON0,
            val,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DIWSTRT; active state changes only when due.
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTRT,
            val,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DIWSTOP; active state changes only when due.
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTOP,
            val,
            1,
            MutationMode::OverwritePending,
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
            1,
            MutationMode::OverwritePending,
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
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPLCON1; active state changes only when due.
    #[inline]
    pub fn set_bplcon1(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON1,
            val,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues CLXCON; active state changes only when due.
    #[inline]
    pub fn set_clxcon(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::CLXCON,
            val,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPLCON2; active state changes only when due.
    #[inline]
    pub fn set_bplcon2(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON2,
            val,
            1,
            MutationMode::OverwritePending,
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
            1,
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
            1,
            MutationMode::OverwritePending,
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
            1,
            MutationMode::OverwritePending,
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
            MutationMode::OverwritePending,
        )
    }

    /// Queues JOYTEST; active state changes only when due.
    #[inline]
    pub fn set_joytest(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::JOYTEST,
            val,
            1,
            MutationMode::OverwritePending,
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

            reg::STREQU | reg::STRVBL | reg::STRHOR | reg::STRLONG | reg::JOYTEST => {}
            _ => {}
        }
    }
}
