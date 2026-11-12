//! Paula register reads, queued writes and private due-event application.

use super::{Paula, BIT_DSKLEN_WRITE_FLAG};
use config::custom_reg::paula as reg;
use config::mask::dskbytr::{
    BIT_DSKBYTR_DISKWRITE, BIT_DSKBYTR_DMAON, BIT_DSKBYTR_DSKBYT, MASK_DSKBYTR_DATA,
};
use config::{audio_disk_control as adkcon, mask::dmacon};
use config::{
    stage_mutation, MutationMode, DMACON_WRITE_DELAY_CCK, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
};

const MASK_PAULA_DMA: u16 = dmacon::BIT_DSKEN | dmacon::MASK_AUD_ALL;
const BIT_DSKLEN_DMA_ENABLE: u16 = 0x8000;

impl Paula {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        self.mutations.iter().flatten().count() < Self::MUTATION_CAPACITY
    }

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

    /// Applies STRHOR at accepted selection.
    #[inline]
    pub fn strobe_strhor(&mut self) {
        self.sync.cck_since_horizontal = 0;
    }

    /// Queues DMACON; active state changes only when due.
    #[inline]
    pub fn set_dmacon(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_DMACON,
            val,
            DMACON_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues INTENA; active state changes only when due.
    #[inline]
    pub fn set_intena(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_INTENA,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues INTREQ; active state changes only when due.
    #[inline]
    pub fn set_intreq(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_INTREQ,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues ADKCON; active state changes only when due.
    #[inline]
    pub fn set_adkcon(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_ADKCON,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DSKDAT; active state changes only when due.
    #[inline]
    pub fn set_dskdat(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_DSKDAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DSKLEN; active state changes only when due.
    #[inline]
    pub fn set_dsklen(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_DSKLEN,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DSKSYNC; active state changes only when due.
    #[inline]
    pub fn set_dsksync(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_DSKSYNC,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues SERDAT; active state changes only when due.
    #[inline]
    pub fn set_serdat(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_SERDAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues SERPER; active state changes only when due.
    #[inline]
    pub fn set_serper(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_SERPER,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues POTGO; active state changes only when due.
    #[inline]
    pub fn set_potgo(&mut self, val: u16) -> bool {
        self.schedule_register(
            reg::REGISTER_POTGO,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the audlen family; active state changes only when due.
    #[inline]
    pub fn set_audlen(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        self.schedule_register(
            reg::REGISTER_AUD0LEN + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the audper family; active state changes only when due.
    #[inline]
    pub fn set_audper(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        self.schedule_register(
            reg::REGISTER_AUD0PER + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the audvol family; active state changes only when due.
    #[inline]
    pub fn set_audvol(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        self.schedule_register(
            reg::REGISTER_AUD0VOL + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the auddat family; active state changes only when due.
    #[inline]
    pub fn set_auddat(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        self.schedule_register(
            reg::REGISTER_AUD0DAT + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Commits a register write directly into active Paula silicon state
    pub(super) fn apply_register_write(&mut self, offset: u16, val: u16) {
        match offset & config::MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_INTENA => self.apply_intena(val),
            reg::REGISTER_INTREQ => self.apply_intreq(val),
            reg::REGISTER_ADKCON => {
                // ADKCON SET/CLR logic
                if (val & adkcon::BIT_SET_CLR) != 0 {
                    self.adkcon |= val & adkcon::MASK_WRITE;
                } else {
                    self.adkcon &= !(val & adkcon::MASK_WRITE);
                }
                self.audio.set_adkcon(self.adkcon);
            }
            reg::REGISTER_DMACON => {
                // DMACON broadcast: update Paula's local channel enables (AUD0..3EN, DSKEN, DMAEN)
                if (val & dmacon::BIT_SET_CLR) != 0 {
                    if (val & dmacon::BIT_DMAEN) != 0 {
                        self.dma_master = true;
                    }
                    self.dma_enables |= val & MASK_PAULA_DMA;
                } else {
                    if (val & dmacon::BIT_DMAEN) != 0 {
                        self.dma_master = false;
                    }
                    self.dma_enables &= !(val & MASK_PAULA_DMA);
                }
                self.audio.set_dma_enables(
                    (self.dma_enables & dmacon::MASK_AUD_ALL) as u8,
                    self.dma_master,
                );
            }
            reg::REGISTER_SERDAT => self.serial_port.write_serdat(val),
            reg::REGISTER_SERPER => self.serial_port.write_serper(val),
            reg::REGISTER_POTGO => self.potgo = val,
            reg::REGISTER_DSKLEN => self.apply_dsklen(val),
            reg::REGISTER_DSKDAT => self.dskdat = val,
            reg::REGISTER_DSKSYNC => self.dsksync = val,

            reg::REGISTER_AUD0LEN => self.audio.set_len(0, val),
            reg::REGISTER_AUD0PER => self.audio.set_per(0, val),
            reg::REGISTER_AUD0VOL => self
                .audio
                .set_vol(0, (val & audio::MASK_VOLUME_REGISTER) as u8),
            reg::REGISTER_AUD0DAT => self.audio.set_dat(0, val),

            reg::REGISTER_AUD1LEN => self.audio.set_len(1, val),
            reg::REGISTER_AUD1PER => self.audio.set_per(1, val),
            reg::REGISTER_AUD1VOL => self
                .audio
                .set_vol(1, (val & audio::MASK_VOLUME_REGISTER) as u8),
            reg::REGISTER_AUD1DAT => self.audio.set_dat(1, val),

            reg::REGISTER_AUD2LEN => self.audio.set_len(2, val),
            reg::REGISTER_AUD2PER => self.audio.set_per(2, val),
            reg::REGISTER_AUD2VOL => self
                .audio
                .set_vol(2, (val & audio::MASK_VOLUME_REGISTER) as u8),
            reg::REGISTER_AUD2DAT => self.audio.set_dat(2, val),

            reg::REGISTER_AUD3LEN => self.audio.set_len(3, val),
            reg::REGISTER_AUD3PER => self.audio.set_per(3, val),
            reg::REGISTER_AUD3VOL => self
                .audio
                .set_vol(3, (val & audio::MASK_VOLUME_REGISTER) as u8),
            reg::REGISTER_AUD3DAT => self.audio.set_dat(3, val),

            reg::REGISTER_STRHOR => self.sync.cck_since_horizontal = 0,
            _ => {}
        }
    }

    /// Reads a Paula register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & config::MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_ADKCONR => self.adkcon,
            reg::REGISTER_POT0DAT => self.pot0dat,
            reg::REGISTER_POT1DAT => self.pot1dat,
            reg::REGISTER_POTGOR => self.potgor,
            reg::REGISTER_SERDATR => self.serial_port.serdatr,
            reg::REGISTER_DSKBYTR => self.peek_dskbytr(),
            reg::REGISTER_INTENAR => self.interrupts.read_intenar(),
            reg::REGISTER_INTREQR => self.interrupts.read_intreqr(),
            _ => config::OPEN_BUS_WORD,
        }
    }

    /// Action method: writes DSKLEN register following the 2-write arming sequence
    fn apply_dsklen(&mut self, val: u16) {
        self.dsklen = val;
        let dmaen = (val & BIT_DSKLEN_DMA_ENABLE) != 0;
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
        self.dskbytr &= !BIT_DSKBYTR_DSKBYT;
        val
    }

    /// Assembles composite live DSKBYTR status from raw read word, DSKLEN, and Paula DMA enables.
    #[inline]
    pub fn assemble_dskbytr(&self, floppy_dskbytr: u16) -> u16 {
        let dmaon = if self.dma_master && (self.dma_enables & dmacon::BIT_DSKEN) != 0 {
            BIT_DSKBYTR_DMAON
        } else {
            0
        };
        let diskwrite = if (self.dsklen & BIT_DSKLEN_WRITE_FLAG) != 0 {
            BIT_DSKBYTR_DISKWRITE
        } else {
            0
        };
        (floppy_dskbytr & MASK_DSKBYTR_DATA) | dmaon | diskwrite
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
