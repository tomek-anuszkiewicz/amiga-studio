//! Motherboard-facing signal state for Agnus.

use crate::{Agnus, AgnusModel};
use config::custom_reg::denise as sync;
use serde::{Deserialize, Serialize};

const FIRST_REFRESH_CCK: u16 = 1;
const SECOND_REFRESH_CCK: u16 = 3;
const NTSC_EQUALIZATION_LINES: u16 = 10;
const PAL_EQUALIZATION_LINES: u16 = 8;
const NTSC_BLANK_LINES: u16 = 21;
const PAL_BLANK_LINES: u16 = 26;

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgnusPins {
    pub incoming_dmal_high: Option<bool>,
    /// Active-low completion pulse from the retained Blitter engine.
    pub int3_high: bool,
    /// CCK edge markers for the existing horizontal/vertical timing sources.
    /// Their widths are not a reconstructed video sync waveform.
    pub hsync_high: bool,
    pub vsync_high: bool,
    /// RGA address driven in this CCK refresh slot; None means no sync strobe.
    pub rga_strobe: Option<u16>,
    /// Delivered active-high DMAL wire; channel decoding is reconstructed in 2.4.
    pub dmal_input_high: bool,
}

impl Default for AgnusPins {
    fn default() -> Self {
        Self {
            incoming_dmal_high: None,
            int3_high: true,
            hsync_high: true,
            vsync_high: true,
            rga_strobe: None,
            dmal_input_high: false,
        }
    }
}

impl Agnus {
    pub(super) fn drive_timing_pins(&mut self, line_ccks: u16, frame_lines: u16) {
        self.pins.hsync_high = self.hpos != line_ccks - 1;
        self.pins.vsync_high = self.vpos != frame_lines - 1 || self.pins.hsync_high;
        self.pins.int3_high = !self.blitter.poll_blit_irq();
    }

    /// Selects the first/second refresh-slot address in the current CCK model.
    /// HRM Appendix A defines slot roles; exact silicon edge calibration is pending.
    pub(super) fn drive_sync_strobe(&mut self) {
        let ntsc = matches!(self.model, AgnusModel::OcsNtsc8370);
        let equalization_lines = if ntsc {
            NTSC_EQUALIZATION_LINES
        } else {
            PAL_EQUALIZATION_LINES + u16::from(self.lof)
        };
        let blank_lines = if ntsc {
            NTSC_BLANK_LINES
        } else {
            PAL_BLANK_LINES
        };
        self.pins.rga_strobe = match self.hpos {
            FIRST_REFRESH_CCK if self.vpos < equalization_lines => Some(sync::STREQU),
            FIRST_REFRESH_CCK if self.vpos < blank_lines => Some(sync::STRVBL),
            FIRST_REFRESH_CCK => Some(sync::STRHOR),
            SECOND_REFRESH_CCK if ntsc && self.lol => Some(sync::STRLONG),
            _ => None,
        };
    }

    /// Captures the motherboard request level without synthesizing a DMA grant.
    #[inline]
    pub fn receive_dmal(&mut self, high: bool) {
        self.pins.dmal_input_high = high;
    }
}

impl Agnus {
    /// Samples the routed request before DMA engine work.
    pub fn sample_input_pins(&mut self) {
        if let Some(high) = self.pins.incoming_dmal_high.take() {
            self.receive_dmal(high);
        }
    }
}
