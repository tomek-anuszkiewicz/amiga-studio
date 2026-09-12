//! Denise (MOS 8362 / 8373) Architecture & Video Display Processor
//!
//! Video pixel serializer, bitplane shifters, Dual Playfield, HAM6, EHB,
//! 32-color palette (COLOR00..COLOR31), hardware sprites, and game port counters.

pub use config::DeniseModel;
use serde::{Deserialize, Serialize};

/// Total number of hardware color palette registers
pub const COLOR_PALETTE_SIZE: usize = 32;

/// Denise video display processor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denise {
    /// Active Denise chip hardware model (8362 OCS, 8373 ECS)
    pub model: DeniseModel,
    /// Bitplane Control 0 (plane count BPU, HIRES, HAM, DBLPF, COLOR)
    pub bplcon0: u16,
    /// Bitplane Control 1 (horizontal scroll offsets for PF1 and PF2)
    pub bplcon1: u16,
    /// Bitplane Control 2 (playfield and sprite priority arbitration)
    pub bplcon2: u16,
    /// Bitplane Control 3 (ECS border color, enhanced sprite control)
    pub bplcon3: u16,
    /// Display Window Start (upper-left corner: VSTART, HSTART)
    pub diwstrt: u16,
    /// Display Window Stop (lower-right corner: VSTOP, HSTOP)
    pub diwstop: u16,
    /// Display Data Fetch Start (bitplane DMA start CCK)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop (bitplane DMA stop CCK)
    pub ddfstop: u16,
    /// Bitplane Modulo 1 (odd bitplanes 1, 3, 5)
    pub bpl1mod: i16,
    /// Bitplane Modulo 2 (even bitplanes 2, 4, 6)
    pub bpl2mod: i16,
    /// 32 palette color registers (12-bit RGB444: 4 bits R, 4 bits G, 4 bits B)
    pub color: [u16; COLOR_PALETTE_SIZE],
    /// Collision Data Register (cleared upon read)
    pub clxdat: u16,
    /// Collision Control Register
    pub clxcon: u16,
    /// Proportional pin drive and start timer (POTGO)
    pub potgo: u16,
}

impl Denise {
    /// Creates a new Denise instance with the specified hardware model
    pub fn new(model: DeniseModel) -> Self {
        Self {
            model,
            bplcon0: 0,
            bplcon1: 0,
            bplcon2: 0,
            bplcon3: 0,
            diwstrt: 0,
            diwstop: 0,
            ddfstrt: 0,
            ddfstop: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            color: [0; COLOR_PALETTE_SIZE],
            clxdat: 0,
            clxcon: 0,
            potgo: 0,
        }
    }

    /// Resets Denise registers to power-on defaults
    pub fn reset(&mut self) {
        self.bplcon0 = 0;
        self.bplcon1 = 0;
        self.bplcon2 = 0;
        self.bplcon3 = 0;
        self.diwstrt = 0;
        self.diwstop = 0;
        self.ddfstrt = 0;
        self.ddfstop = 0;
        self.bpl1mod = 0;
        self.bpl2mod = 0;
        self.color.fill(0);
        self.clxdat = 0;
        self.clxcon = 0;
        self.potgo = 0;
    }

    /// Advances Denise pixel pipeline by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: pixel serialization & rasterization
    }

    /// Reads collision data register (CLXDAT at $DFF00E) and clears it on read
    #[inline]
    pub fn read_clxdat(&mut self) -> u16 {
        let val = self.clxdat;
        self.clxdat = 0;
        val
    }

    /// Writes to color palette register (COLOR00..COLOR31 at $DFF180..$DFF1BE)
    #[inline]
    pub fn write_color(&mut self, index: usize, val: u16) {
        if index < COLOR_PALETTE_SIZE {
            self.color[index] = val & 0x0FFF; // 12-bit RGB444
        }
    }

    /// Reads color palette register (12-bit RGB444)
    #[inline]
    pub fn read_color(&self, index: usize) -> u16 {
        if index < COLOR_PALETTE_SIZE {
            self.color[index]
        } else {
            0
        }
    }
}
