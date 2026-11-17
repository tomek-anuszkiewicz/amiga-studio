//! HRM chapter 4 control/data stream and per-channel vertical DMA latches.

use crate::{Agnus, AgnusModel};
use config::mask::dmacon;
use config::{chip_ram, custom_reg::denise as reg, SPRITE_COUNT, SPRITE_REGISTER_STRIDE};
use serde::{Deserialize, Serialize};

const PAL_CONTROL_FETCH_LINE: u16 = 25;
const NTSC_CONTROL_FETCH_LINE: u16 = 19;
const BIT_START_HIGH: u16 = 0x0004;
const BIT_STOP_HIGH: u16 = 0x0002;
const WORD_BYTES: u32 = 2;
const SECOND_WORD_PHASE: u16 = 2;
const MASK_ALL_SPRITES: u8 = u8::MAX;

/// Committed sprite coordinates and vertical DMA latches.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteDmaState {
    pub sprpos: [u16; SPRITE_COUNT],
    pub sprctl: [u16; SPRITE_COUNT],
    pub sprite_dma_active: [bool; SPRITE_COUNT],
}

/// Vertical comparisons remain valid until their registers or beam line change.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "SpriteDmaState", into = "SpriteDmaState")]
pub struct SpriteDma {
    sprpos: [u16; SPRITE_COUNT],
    sprctl: [u16; SPRITE_COUNT],
    active: u8,
    compared_line: Option<(u16, u16)>,
    requests: u8,
}

impl PartialEq for SpriteDma {
    fn eq(&self, other: &Self) -> bool {
        self.sprpos == other.sprpos && self.sprctl == other.sprctl && self.active == other.active
    }
}

impl Eq for SpriteDma {}

impl From<SpriteDmaState> for SpriteDma {
    fn from(state: SpriteDmaState) -> Self {
        Self {
            sprpos: state.sprpos,
            sprctl: state.sprctl,
            active: state
                .sprite_dma_active
                .iter()
                .enumerate()
                .fold(0, |mask, (sprite, &active)| {
                    mask | (u8::from(active) << sprite)
                }),
            ..Self::default()
        }
    }
}

impl From<SpriteDma> for SpriteDmaState {
    fn from(dma: SpriteDma) -> Self {
        dma.state()
    }
}

impl SpriteDma {
    pub fn state(&self) -> SpriteDmaState {
        SpriteDmaState {
            sprpos: self.sprpos,
            sprctl: self.sprctl,
            sprite_dma_active: core::array::from_fn(|sprite| self.active & (1 << sprite) != 0),
        }
    }

    #[inline]
    pub fn set_position(&mut self, sprite: usize, value: u16) {
        self.sprpos[sprite] = value;
        self.compared_line = None;
    }

    #[inline]
    pub fn set_control(&mut self, sprite: usize, value: u16) {
        self.sprctl[sprite] = value;
        self.compared_line = None;
    }

    #[inline]
    fn update(&mut self, vpos: u16, first_line: u16) {
        if self.compared_line != Some((vpos, first_line)) {
            self.compare(vpos, first_line);
        }
    }

    fn compare(&mut self, vpos: u16, first_line: u16) {
        self.compared_line = Some((vpos, first_line));
        if vpos <= first_line {
            self.active = 0;
            self.requests = if vpos == first_line {
                MASK_ALL_SPRITES
            } else {
                0
            };
        } else {
            self.requests = 0;
            for sprite in 0..SPRITE_COUNT {
                let bit = 1 << sprite;
                let pos = self.sprpos[sprite];
                let ctl = self.sprctl[sprite];
                let start = (pos >> 8) | ((ctl & BIT_START_HIGH) << 6);
                let stop = (ctl >> 8) | ((ctl & BIT_STOP_HIGH) << 7);
                let started = self.active & bit != 0 || vpos == start;
                let stopped = vpos == stop;
                self.active = (self.active & !bit) | (u8::from(started && !stopped) << sprite);
                self.requests |= u8::from(started || stopped) << sprite;
            }
        }
    }
}

impl Agnus {
    fn sprite_control_line(&self) -> u16 {
        match self.model {
            AgnusModel::OcsPal8371 => PAL_CONTROL_FETCH_LINE,
            AgnusModel::OcsNtsc8370 => NTSC_CONTROL_FETCH_LINE,
        }
    }

    #[inline]
    fn sprite_stop(&self, sprite: usize) -> u16 {
        let ctl = self.sprite_dma.sprctl[sprite];
        (ctl >> 8) | ((ctl & BIT_STOP_HIGH) << 7)
    }

    #[inline]
    pub(super) fn sprite_requests(&mut self) -> u8 {
        let first_line = self.sprite_control_line();
        self.sprite_dma.update(self.vpos, first_line);
        if self.sprite_dma.requests == 0 {
            return 0;
        }
        if self.is_dma_enabled(dmacon::BIT_SPREN) && self.pending_dma_write.is_none() {
            self.sprite_dma.requests
        } else {
            0
        }
    }

    pub(super) fn fetch_sprite_word(
        &mut self,
        sprite: usize,
        ram: &mut impl config::chip_ram::ChipRamAccess,
    ) {
        if self.pending_dma_write.is_some() {
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
        let word = ram.read_chip_word(self.sprpt[sprite]);
        self.sprpt[sprite] =
            self.sprpt[sprite].wrapping_add(WORD_BYTES) & chip_ram::MASK_OCS_DMA_ADDRESS;
        self.pending_dma_write = Some((base + sprite as u16 * SPRITE_REGISTER_STRIDE, word));
    }
}
