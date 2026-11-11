//! Queued Paula register setters and private due-event application.

use super::{Paula, INTERRUPT_INPUT};
use config::custom_reg::paula as reg;
use config::{stage_mutation, MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Paula {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        self.mutations.iter().flatten().count() < Self::MUTATION_CAPACITY - 1
    }

    // Reserve one entry for the single merged interrupt-input event.
    fn schedule_register(
        &mut self,
        offset: u16,
        value: u16,
        delay: u8,
        mode: MutationMode,
    ) -> bool {
        self.can_accept_register_write()
            && stage_mutation(&mut self.mutations, offset, value, delay, mode)
    }

    /// Queues STRHOR; active state changes only when due.
    #[inline]
    pub fn set_strhor(&mut self) -> bool {
        self.schedule_register(
            reg::STRHOR,
            0,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DMACON; active state changes only when due.
    #[inline]
    pub fn set_dmacon(&mut self, val: u16) -> bool {
        self.schedule_register(reg::DMACON, val, 2, MutationMode::OverwritePending)
    }

    /// Queues INTENA; active state changes only when due.
    #[inline]
    pub fn set_intena(&mut self, val: u16) -> bool {
        self.schedule_register(reg::INTENA, val, 1, MutationMode::OverwritePending)
    }

    /// Queues INTREQ; active state changes only when due.
    #[inline]
    pub fn set_intreq(&mut self, val: u16) -> bool {
        self.schedule_register(reg::INTREQ, val, 1, MutationMode::OverwritePending)
    }

    /// Queues ADKCON; active state changes only when due.
    #[inline]
    pub fn set_adkcon(&mut self, val: u16) -> bool {
        self.schedule_register(reg::ADKCON, val, 2, MutationMode::OverwritePending)
    }

    /// Queues DSKDAT; active state changes only when due.
    #[inline]
    pub fn set_dskdat(&mut self, val: u16) -> bool {
        self.schedule_register(reg::DSKDAT, val, 2, MutationMode::OverwritePending)
    }

    /// Queues DSKLEN; active state changes only when due.
    #[inline]
    pub fn set_dsklen(&mut self, val: u16) -> bool {
        self.schedule_register(reg::DSKLEN, val, 2, MutationMode::OverwritePending)
    }

    /// Queues DSKSYNC; active state changes only when due.
    #[inline]
    pub fn set_dsksync(&mut self, val: u16) -> bool {
        self.schedule_register(reg::DSKSYNC, val, 2, MutationMode::OverwritePending)
    }

    /// Queues SERDAT; active state changes only when due.
    #[inline]
    pub fn set_serdat(&mut self, val: u16) -> bool {
        self.schedule_register(reg::SERDAT, val, 1, MutationMode::Pipeline)
    }

    /// Queues SERPER; active state changes only when due.
    #[inline]
    pub fn set_serper(&mut self, val: u16) -> bool {
        self.schedule_register(reg::SERPER, val, 2, MutationMode::OverwritePending)
    }

    /// Queues POTGO; active state changes only when due.
    #[inline]
    pub fn set_potgo(&mut self, val: u16) -> bool {
        self.schedule_register(reg::POTGO, val, 2, MutationMode::OverwritePending)
    }

    /// Queues the audlen family; active state changes only when due.
    #[inline]
    pub fn set_audlen(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        self.schedule_register(
            reg::AUD0LEN + (index as u16) * 16,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the audper family; active state changes only when due.
    #[inline]
    pub fn set_audper(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        self.schedule_register(
            reg::AUD0PER + (index as u16) * 16,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the audvol family; active state changes only when due.
    #[inline]
    pub fn set_audvol(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        self.schedule_register(
            reg::AUD0VOL + (index as u16) * 16,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the auddat family; active state changes only when due.
    #[inline]
    pub fn set_auddat(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        self.schedule_register(
            reg::AUD0DAT + (index as u16) * 16,
            val,
            1,
            MutationMode::Pipeline,
        )
    }

    /// Commits a register write directly into active Paula silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            reg::INTENA => self.apply_intena(val),
            reg::INTREQ | INTERRUPT_INPUT => self.apply_intreq(val),
            reg::ADKCON => {
                // ADKCON SET/CLR logic
                if (val & 0x8000) != 0 {
                    self.adkcon |= val & 0x7FFF;
                } else {
                    self.adkcon &= !(val & 0x7FFF);
                }
                self.audio.set_adkcon(self.adkcon);
            }
            reg::DMACON => {
                // DMACON broadcast: update Paula's local channel enables (AUD0..3EN, DSKEN, DMAEN)
                if (val & 0x8000) != 0 {
                    if (val & 0x0200) != 0 {
                        self.dma_master = true;
                    }
                    self.dma_enables |= val & 0x001F;
                } else {
                    if (val & 0x0200) != 0 {
                        self.dma_master = false;
                    }
                    self.dma_enables &= !(val & 0x001F);
                }
                self.audio
                    .set_dma_enables((self.dma_enables & 0x000F) as u8, self.dma_master);
            }
            reg::SERDAT => self.serial_port.write_serdat(val),
            reg::SERPER => self.serial_port.write_serper(val),
            reg::POTGO => self.potgo = val,
            reg::DSKLEN => self.apply_dsklen(val),
            reg::DSKDAT => self.dskdat = val,
            reg::DSKSYNC => self.dsksync = val,

            reg::AUD0LEN => self.audio.set_len(0, val),
            reg::AUD0PER => self.audio.set_per(0, val),
            reg::AUD0VOL => self.audio.set_vol(0, (val & 0x007F) as u8),
            reg::AUD0DAT => self.audio.set_dat(0, val),

            reg::AUD1LEN => self.audio.set_len(1, val),
            reg::AUD1PER => self.audio.set_per(1, val),
            reg::AUD1VOL => self.audio.set_vol(1, (val & 0x007F) as u8),
            reg::AUD1DAT => self.audio.set_dat(1, val),

            reg::AUD2LEN => self.audio.set_len(2, val),
            reg::AUD2PER => self.audio.set_per(2, val),
            reg::AUD2VOL => self.audio.set_vol(2, (val & 0x007F) as u8),
            reg::AUD2DAT => self.audio.set_dat(2, val),

            reg::AUD3LEN => self.audio.set_len(3, val),
            reg::AUD3PER => self.audio.set_per(3, val),
            reg::AUD3VOL => self.audio.set_vol(3, (val & 0x007F) as u8),
            reg::AUD3DAT => self.audio.set_dat(3, val),

            reg::STRHOR => {}
            _ => {}
        }
    }
}
