//! HRM chapter 4 control/data stream and per-channel vertical DMA latches.

use crate::{Agnus, AgnusModel};
use config::{chip_ram, custom_reg::denise as reg, SPRITE_COUNT, SPRITE_REGISTER_STRIDE};

const PAL_CONTROL_FETCH_LINE: u16 = 25;
const NTSC_CONTROL_FETCH_LINE: u16 = 19;
const BIT_START_HIGH: u16 = 0x0004;
const BIT_STOP_HIGH: u16 = 0x0002;
const WORD_BYTES: u32 = 2;
const SECOND_WORD_PHASE: u16 = 2;

impl Agnus {
    fn sprite_control_line(&self) -> u16 {
        match self.model {
            AgnusModel::OcsPal8371 => PAL_CONTROL_FETCH_LINE,
            AgnusModel::OcsNtsc8370 => NTSC_CONTROL_FETCH_LINE,
        }
    }

    #[inline]
    fn sprite_stop(&self, sprite: usize) -> u16 {
        (self.sprctl[sprite] >> 8) | ((self.sprctl[sprite] & BIT_STOP_HIGH) << 7)
    }

    pub(super) fn sprite_requests(&mut self) -> [bool; SPRITE_COUNT] {
        let first_line = self.sprite_control_line();
        let output_available = self.pending_dma_write.is_none();
        if self.vpos <= first_line {
            self.sprite_dma_active.fill(false);
            return [self.vpos == first_line && output_available; SPRITE_COUNT];
        }
        let mut requests = [false; SPRITE_COUNT];
        for (sprite, request) in requests.iter_mut().enumerate() {
            let start = (self.sprpos[sprite] >> 8) | ((self.sprctl[sprite] & BIT_START_HIGH) << 6);
            let started = self.sprite_dma_active[sprite] | (self.vpos == start);
            let stopped = self.vpos == self.sprite_stop(sprite);
            // Stop wins the latch comparison, but still requests the next control pair.
            self.sprite_dma_active[sprite] = started & !stopped;
            *request = (started | stopped) & output_available;
        }
        requests
    }

    pub(super) fn fetch_sprite_word(&mut self, sprite: usize, ram: &[u8]) {
        if ram.is_empty() || self.pending_dma_write.is_some() {
            return;
        }
        let control =
            self.vpos == self.sprite_control_line() || self.vpos == self.sprite_stop(sprite);
        let second = (self.hpos - crate::dma::HPOS_SPRITE_START)
            % crate::dma::SPRITE_SLOT_PERIOD_CCK
            == SECOND_WORD_PHASE;
        let base = match (control, second) {
            (true, false) => reg::REGISTER_SPR0POS,
            (true, true) => reg::REGISTER_SPR0CTL,
            (false, false) => reg::REGISTER_SPR0DATA,
            (false, true) => reg::REGISTER_SPR0DATB,
        };
        let word = chip_ram::read_u16(ram, self.sprpt[sprite]);
        self.sprpt[sprite] =
            self.sprpt[sprite].wrapping_add(WORD_BYTES) & chip_ram::MASK_OCS_DMA_ADDRESS;
        self.pending_dma_write = Some((base + sprite as u16 * SPRITE_REGISTER_STRIDE, word));
    }
}
