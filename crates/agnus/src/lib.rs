//! Agnus (MOS 8370 / 8371 / 8372A) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub use config::AgnusModel;
use serde::{Deserialize, Serialize};

/// Maximum horizontal Color Clock cycles per scanline (PAL)
pub const PAL_LINE_CCKS: u16 = 227;
/// Total vertical scanlines per frame (PAL)
pub const PAL_FRAME_LINES: u16 = 312;
/// Maximum horizontal Color Clock cycles per scanline (NTSC)
pub const NTSC_LINE_CCKS: u16 = 227;
/// Total vertical scanlines per frame (NTSC)
pub const NTSC_FRAME_LINES: u16 = 262;

/// Agnus custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agnus {
    /// Active Agnus chip hardware model (8370, 8371, 8372A)
    pub model: AgnusModel,
    /// Horizontal raster beam counter (0..227 CCK)
    pub hpos: u16,
    /// Vertical scanline counter (0..312 PAL, 0..262 NTSC)
    pub vpos: u16,
    /// Long Frame toggle bit (toggled every field in interlace mode)
    pub lof: bool,
    /// True if Chip RAM is currently blocked by custom chip DMA
    pub chip_ram_blocked: bool,
}

impl Agnus {
    /// Creates a new Agnus instance with specified chip revision
    pub fn new(model: AgnusModel) -> Self {
        Self {
            model,
            hpos: 0,
            vpos: 0,
            lof: false,
            chip_ram_blocked: false,
        }
    }

    /// Resets Agnus registers and beam counters to power-on defaults
    pub fn reset(&mut self) {
        self.hpos = 0;
        self.vpos = 0;
        self.lof = false;
        self.chip_ram_blocked = false;
    }

    /// Advances raster beam position by 1 Color Clock
    pub fn step_cck(&mut self) {
        let max_lines = match self.model {
            AgnusModel::OcsNtsc8370 => NTSC_FRAME_LINES,
            _ => PAL_FRAME_LINES,
        };

        // Advance horizontal beam counter
        self.hpos = self.hpos.wrapping_add(1);
        if self.hpos > PAL_LINE_CCKS {
            self.hpos = 0;
            self.vpos = self.vpos.wrapping_add(1);
            if self.vpos >= max_lines {
                self.vpos = 0;
                self.lof = !self.lof;
            }
        }
    }

    /// Returns true if Chip RAM is currently locked by custom chip DMA
    #[inline]
    pub fn is_chip_ram_blocked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Reads Vertical & Horizontal beam position (VHPOSR at $DFF004)
    #[inline]
    pub fn vhposr(&self) -> u16 {
        let v_low = (self.vpos & 0xFF) as u16;
        let h = (self.hpos & 0xFF) as u16;
        (v_low << 8) | h
    }

    /// Reads Vertical beam position high bit and chip ID (VPOSR at $DFF006)
    #[inline]
    pub fn vposr(&self) -> u16 {
        let mut val = 0u16;
        if self.lof {
            val |= 0x8000;
        }
        match self.model {
            AgnusModel::OcsNtsc8370 => val |= 0x1000, // NTSC chip ID
            AgnusModel::OcsPal8371 => {}              // OCS PAL chip ID = 0
        }
        val |= (self.vpos >> 8) & 0x07;
        val
    }
}
