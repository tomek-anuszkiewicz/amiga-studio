//! Receive audio request pairs on DMAL, then supply one granted AUDxDAT word.

use crate::Agnus;
use config::{chip_ram, custom_reg, dmal, AUDIO_CHANNEL_COUNT};

impl Agnus {
    /// Called after beam advancement; this input was driven in the preceding CCK.
    pub(super) fn sample_audio_request(&mut self) {
        let position = self.hpos.wrapping_sub(1);
        if !(dmal::FIRST_AUDIO_BIT..dmal::FRAME_BITS).contains(&position) {
            return;
        }
        let offset = position - dmal::FIRST_AUDIO_BIT;
        let channel = usize::from(offset / dmal::PAIR_BITS);
        if offset % dmal::PAIR_BITS == dmal::REQUEST_BIT {
            self.audio_dma_request[channel] = self.pins.dmal_input_high;
        } else {
            self.audio_dma_restart[channel] = self.pins.dmal_input_high;
        }
    }

    pub(super) fn audio_requests(&self) -> [bool; AUDIO_CHANNEL_COUNT] {
        if self.pending_dma_write.is_none() {
            self.audio_dma_request
        } else {
            [false; AUDIO_CHANNEL_COUNT]
        }
    }

    pub(super) fn fetch_audio_word(&mut self, channel: usize, ram: &[u8]) {
        if ram.is_empty() || self.pending_dma_write.is_some() {
            return;
        }
        let address = if self.audio_dma_restart[channel] {
            self.audlc[channel]
        } else {
            self.audpt[channel]
        };
        let word = chip_ram::read_u16(ram, address);
        let register =
            custom_reg::paula::REGISTER_AUD0DAT + channel as u16 * config::AUDIO_REGISTER_STRIDE;
        self.pending_dma_write = Some((register, word));
        self.audpt[channel] = address.wrapping_add(2) & chip_ram::MASK_OCS_DMA_ADDRESS;
        self.audio_dma_request[channel] = false;
        self.audio_dma_restart[channel] = false;
    }
}
