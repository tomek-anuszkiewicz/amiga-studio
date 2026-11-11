//! Paula register reads, queued writes and private due-event application.

use super::{
    Paula, DSKBYTR_DATA_MASK, DSKBYTR_DISKWRITE, DSKBYTR_DMAON, DSKBYTR_DSKBYT, DSKBYTR_DSKEN,
    DSKLEN_WRITE_FLAG, INTERRUPT_INPUT,
};
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

    /// Reads a Paula register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            reg::ADKCONR => self.adkcon,
            reg::POT0DAT => self.pot0dat,
            reg::POT1DAT => self.pot1dat,
            reg::POTGOR => self.potgor,
            reg::SERDATR => self.serial_port.serdatr,
            reg::DSKBYTR => self.peek_dskbytr(),
            reg::INTENAR => self.interrupts.read_intenar(),
            reg::INTREQR => self.interrupts.read_intreqr(),
            _ => config::OPEN_BUS_WORD,
        }
    }

    /// Action method: writes DSKLEN register following the 2-write arming sequence
    fn apply_dsklen(&mut self, val: u16) {
        self.dsklen = val;
        let dmaen = (val & 0x8000) != 0;
        if !dmaen {
            self.dma_armed = false;
            self.dma_active = false;
        } else if !self.dma_armed {
            self.dma_armed = true;
        } else {
            self.dma_active = true;
        }
    }

    /// Returns composite DSKBYTR status with live flags without side effects (for debuggers and peek inspection)
    #[inline]
    pub fn peek_dskbytr(&self) -> u16 {
        self.assemble_dskbytr(self.dskbytr)
    }

    /// Reads composite live DSKBYTR with Clear-on-Read hardware side-effects (clears bit 15 DSKBYT)
    #[inline]
    pub fn read_dskbytr(&mut self) -> u16 {
        let val = self.peek_dskbytr();
        self.dskbytr &= !DSKBYTR_DSKBYT;
        val
    }

    /// Assembles composite live DSKBYTR status from raw read word, DSKLEN, and Paula DMA enables.
    #[inline]
    pub fn assemble_dskbytr(&self, floppy_dskbytr: u16) -> u16 {
        let dmaon = if self.dma_master && (self.dma_enables & DSKBYTR_DSKEN) != 0 {
            DSKBYTR_DMAON
        } else {
            0
        };
        let diskwrite = if (self.dsklen & DSKLEN_WRITE_FLAG) != 0 {
            DSKBYTR_DISKWRITE
        } else {
            0
        };
        (floppy_dskbytr & DSKBYTR_DATA_MASK) | dmaon | diskwrite
    }

    /// Writes INTENA register following SET/CLR bit 15 logic
    #[inline]
    fn apply_intena(&mut self, val: u16) {
        self.interrupts.write_intena(val);
    }

    /// Writes INTREQ register following SET/CLR bit 15 logic
    #[inline]
    fn apply_intreq(&mut self, val: u16) {
        self.interrupts.write_intreq(val);
    }

    /// Reads Audio/Disk Control register (ADKCONR at $DFF010)
    #[inline(always)]
    pub fn adkconr(&self) -> u16 {
        self.adkcon
    }

    /// Reads Audio/Disk Control register without side-effects for debugging
    #[inline(always)]
    pub fn adkconr_debug(&self) -> u16 {
        self.adkcon
    }

    /// Reads Potentiometer 0 data register (POT0DAT at $DFF012)
    #[inline(always)]
    pub fn pot0dat(&self) -> u16 {
        self.pot0dat
    }

    /// Reads Potentiometer 0 data register without side-effects for debugging
    #[inline(always)]
    pub fn pot0dat_debug(&self) -> u16 {
        self.pot0dat
    }

    /// Reads Potentiometer 1 data register (POT1DAT at $DFF014)
    #[inline(always)]
    pub fn pot1dat(&self) -> u16 {
        self.pot1dat
    }

    /// Reads Potentiometer 1 data register without side-effects for debugging
    #[inline(always)]
    pub fn pot1dat_debug(&self) -> u16 {
        self.pot1dat
    }

    /// Reads Potentiometer Port control/data register (POTGOR at $DFF016)
    #[inline(always)]
    pub fn potgor(&self) -> u16 {
        self.potgor
    }

    /// Reads Potentiometer Port control/data register without side-effects for debugging
    #[inline(always)]
    pub fn potgor_debug(&self) -> u16 {
        self.potgor
    }

    /// Reads Serial port data and status register (SERDATR at $DFF018)
    #[inline(always)]
    pub fn serdatr(&self) -> u16 {
        self.serial_port.serdatr
    }

    /// Reads Serial port data and status register without side-effects for debugging
    #[inline(always)]
    pub fn serdatr_debug(&self) -> u16 {
        self.serial_port.serdatr
    }

    /// Reads Interrupt enable register (INTENAR at $DFF01C)
    #[inline(always)]
    pub fn intenar(&self) -> u16 {
        self.interrupts.read_intenar()
    }

    /// Reads Interrupt enable register without side-effects for debugging
    #[inline(always)]
    pub fn intenar_debug(&self) -> u16 {
        self.interrupts.read_intenar()
    }

    /// Reads Interrupt request register (INTREQR at $DFF01E)
    #[inline(always)]
    pub fn intreqr(&self) -> u16 {
        self.interrupts.read_intreqr()
    }

    /// Reads Interrupt request register without side-effects for debugging
    #[inline(always)]
    pub fn intreqr_debug(&self) -> u16 {
        self.interrupts.read_intreqr()
    }
}
