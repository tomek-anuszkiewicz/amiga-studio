//! Denise (MOS 8362 / 8373) Architecture & Video Display Processor
//!
//! Video pixel serializer, bitplane shifters, Dual Playfield, HAM6, EHB,
//! 32-color palette (COLOR00..COLOR31), hardware sprites, and game port counters.

pub use frame_builder;
pub mod sprites;
pub use sprites::Sprites;

pub use config::DeniseModel;
use config::{tick_mutations, DelayedMutation};
use serde::{Deserialize, Serialize};

mod game_ports;
mod pins;
pub use pins::{DeniseInputs, DenisePins, DeniseSync};

mod registers;
mod render;

const MASK_HORIZONTAL_CURSOR: u16 = 0xff;

pub mod decode;
pub use decode::{decode_dual_playfield, decode_ehb, decode_ham6};

const PIPELINE_PIXELS: usize = 2;
const MASK_SCROLL_AMOUNT: u16 = 0x000f;
const BIT_EVEN_SCROLL_SHIFT: u16 = 4;
const BIT_BITPLANE_SERIAL: u16 = 0x8000;
const MASK_ALL_PLANE_PIXEL: u8 = 0x3F;
const BIT_PF2_PRIORITY: u16 = 0x0040;

/// Denise video display processor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denise {
    /// Motherboard-facing signal state.
    pub pins: DenisePins,
    /// Receiver-owned synchronization and rendering cursor.
    pub sync: DeniseSync,
    /// Active Denise chip hardware model (8362 OCS, 8373 ECS)
    pub model: DeniseModel,
    /// Denise 8 hardware sprite engines
    pub sprites: sprites::Sprites,
    /// Storage and field assembly for the pixels produced by Denise.
    pub frame_builder: frame_builder::FrameBuilder,

    // --- Active Latched Registers (Read is NOW) ---
    /// Bitplane Control 0 (plane count BPU, HIRES, HAM, DBLPF, COLOR)
    pub bplcon0: u16,
    /// Bitplane Control 1 (horizontal scroll offsets for PF1 and PF2)
    pub bplcon1: u16,
    /// Bitplane Control 2 (playfield and sprite priority arbitration)
    pub bplcon2: u16,
    /// Display Window Start ($08E, upper-left corner: VSTART, HSTART)
    pub diwstrt: u16,
    pub diwstop: u16,
    /// Internal Denise horizontal display window flip-flop
    pub hflop: bool,
    /// Display Data Fetch Start ($092)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094)
    pub ddfstop: u16,
    /// Bitplane Modulo 1 ($108, odd bitplanes 1, 3, 5)
    pub bpl1mod: i16,
    /// Bitplane Modulo 2 ($10A, even bitplanes 2, 4, 6)
    pub bpl2mod: i16,
    /// 32 palette color registers (12-bit RGB444: 4 bits R, 4 bits G, 4 bits B)
    pub color: [u16; Denise::COLOR_PALETTE_SIZE],
    /// Collision Data Register (cleared upon read)
    pub clxdat: u16,
    /// Collision Control Register
    pub clxcon: u16,
    /// Proportional pin drive and start timer (POTGO)
    pub potgo: u16,

    /// Joystick / Mouse Port 1 counter ($00A)
    pub joy0dat: u16,
    /// Joystick / Mouse Port 2 counter ($00C)
    pub joy1dat: u16,

    /// Bitplane data latches ($110-$11A)
    pub bpldat: [u16; config::OCS_BITPLANE_COUNT],
    /// Bitplane pipeline holding registers feeding the shifters
    pub bpldat_pipe: [u16; config::OCS_BITPLANE_COUNT],
    /// Active parallel bitplane shift registers
    pub shifters: [u16; config::OCS_BITPLANE_COUNT],
    /// Pixel delay history for the odd/even BPLCON1 scroll controls.
    pub scroll_history: [u16; config::OCS_BITPLANE_COUNT],
    /// True when BPL1DAT has been written on the active scanline, arming bitplane serialization
    pub bpl_armed: bool,
    /// Last held color for HAM6 mode
    pub last_ham_rgb: u16,
    /// Pipeline delayed pixels (2 hires pixels hardware latency)
    pub pipeline_pixels: [u32; PIPELINE_PIXELS],
    pub pipeline_pixels_valid: [bool; PIPELINE_PIXELS],

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; Denise::MUTATION_CAPACITY],
}

impl Denise {
    /// Total number of hardware color palette registers
    pub const COLOR_PALETTE_SIZE: usize = 32;

    /// Fixed-capacity in-flight register mutation buffer for Denise (covers 32 colors + controls)
    pub const MUTATION_CAPACITY: usize = 64;

    /// Creates a new Denise instance with the specified hardware model
    pub fn new(model: DeniseModel) -> Self {
        Self {
            pins: DenisePins::default(),
            sync: DeniseSync::default(),
            model,
            sprites: sprites::Sprites::new(),
            frame_builder: frame_builder::FrameBuilder::new(),
            bplcon0: 0,
            bplcon1: 0,
            bplcon2: 0,
            diwstrt: 0,
            diwstop: 0,
            hflop: false,
            ddfstrt: 0,
            ddfstop: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            color: [0; Self::COLOR_PALETTE_SIZE],
            clxdat: 0,
            clxcon: 0,
            potgo: 0,
            joy0dat: 0,
            joy1dat: 0,
            bpldat: [0; config::OCS_BITPLANE_COUNT],
            bpldat_pipe: [0; config::OCS_BITPLANE_COUNT],
            shifters: [0; config::OCS_BITPLANE_COUNT],
            scroll_history: [0; config::OCS_BITPLANE_COUNT],
            bpl_armed: false,
            last_ham_rgb: 0,
            pipeline_pixels: [0; PIPELINE_PIXELS],
            pipeline_pixels_valid: [false; PIPELINE_PIXELS],
            mutations: [None; Self::MUTATION_CAPACITY],
        }
    }

    /// Resets Denise registers to power-on defaults
    pub fn reset(&mut self) {
        self.pins = DenisePins::default();
        self.sync = DeniseSync::default();
        self.sprites.reset();
        self.frame_builder.reset();
        self.bplcon0 = 0;
        self.bplcon1 = 0;
        self.bplcon2 = 0;
        self.diwstrt = 0;
        self.diwstop = 0;
        self.hflop = false;
        self.ddfstrt = 0;
        self.ddfstop = 0;
        self.bpl1mod = 0;
        self.bpl2mod = 0;
        self.color.fill(0);
        self.clxdat = 0;
        self.clxcon = 0;
        self.potgo = 0;
        self.joy0dat = 0;
        self.joy1dat = 0;
        self.bpldat.fill(0);
        self.bpldat_pipe.fill(0);
        self.shifters.fill(0);
        self.scroll_history.fill(0);
        self.bpl_armed = false;
        self.last_ham_rgb = 0;
        self.pipeline_pixels = [0; PIPELINE_PIXELS];
        self.pipeline_pixels_valid = [false; PIPELINE_PIXELS];
        self.mutations = [None; Self::MUTATION_CAPACITY];
    }

    /// Advances Denise pixel pipeline, steps sprites and frame builder,
    /// and processes in-flight mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; Self::MUTATION_CAPACITY] {
        if let Some(address) = self.pins.rga_strobe.take() {
            self.receive_sync_strobe(address);
        }
        self.step_game_ports();
        let hpos = self.sync.hpos;
        let vpos = self.sync.vpos;
        self.sync.hpos = self.sync.hpos.wrapping_add(1) & MASK_HORIZONTAL_CURSOR;
        self.render_cck(hpos, vpos);

        let mut committed = [None; Self::MUTATION_CAPACITY];
        let mut committed_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if committed_count < committed.len() {
                committed[committed_count] = Some((reg, val));
                committed_count += 1;
            }
        });
        for item in committed[..committed_count].iter().flatten() {
            self.apply_register_write(item.0, item.1);
        }

        committed
    }

    /// Shifts out 1 pixel (MSB first) across all 6 bitplanes
    #[inline]
    pub fn shift_pixel(&mut self) -> u8 {
        let mut pixel = 0;
        for (plane, shifter) in self.shifters.iter_mut().enumerate() {
            let bit = u16::from(*shifter & BIT_BITPLANE_SERIAL != 0);
            *shifter <<= 1;
            self.scroll_history[plane] = (self.scroll_history[plane] << 1) | bit;
            let scroll = if plane % 2 == 0 {
                self.bplcon1 & MASK_SCROLL_AMOUNT
            } else {
                (self.bplcon1 >> BIT_EVEN_SCROLL_SHIFT) & MASK_SCROLL_AMOUNT
            };
            pixel |= (((self.scroll_history[plane] >> scroll) & 1) as u8) << plane;
        }
        pixel
    }

    /// Decodes a 6-bit pixel into 12-bit RGB444 based on active display mode
    pub fn decode_pixel(&mut self, planes_data: u8) -> u16 {
        let plane_count = self.bitplane_count();
        if plane_count == 0 {
            return self.color[0];
        }

        let mask = if plane_count >= config::OCS_BITPLANE_COUNT as u8 {
            MASK_ALL_PLANE_PIXEL
        } else {
            (1 << plane_count) - 1
        };
        let data = planes_data & mask;

        if self.is_ham() && !self.is_hires() && plane_count == config::OCS_BITPLANE_COUNT as u8 {
            decode_ham6(data, &self.color, &mut self.last_ham_rgb)
        } else if self.is_dual_playfield() {
            let pf2_priority = (self.bplcon2 & BIT_PF2_PRIORITY) != 0;
            decode_dual_playfield(data, &self.color, pf2_priority)
        } else if plane_count == config::OCS_BITPLANE_COUNT as u8 && !self.is_hires() {
            decode_ehb(data, &self.color)
        } else {
            let idx = (data & decode::MASK_PALETTE_INDEX) as usize;
            self.color[idx]
        }
    }

    /// Returns the active number of bitplanes (0..6)
    #[inline]
    pub fn bitplane_count(&self) -> u8 {
        ((self.bplcon0 >> config::mask::bplcon0::BIT_BPU_SHIFT)
            & (config::mask::bplcon0::MASK_BPU >> config::mask::bplcon0::BIT_BPU_SHIFT))
            as u8
    }

    /// Returns true if High-Resolution (640-pixel) mode is enabled
    #[inline]
    pub fn is_hires(&self) -> bool {
        (self.bplcon0 & config::mask::bplcon0::BIT_HIRES) != 0
    }

    /// Returns true if Hold-And-Modify (HAM) mode is enabled
    #[inline]
    fn is_ham(&self) -> bool {
        (self.bplcon0 & config::mask::bplcon0::BIT_HOMOD) != 0
    }

    /// Returns true if Dual Playfield mode is enabled
    #[inline]
    fn is_dual_playfield(&self) -> bool {
        (self.bplcon0 & config::mask::bplcon0::BIT_DBLPF) != 0
    }
}
