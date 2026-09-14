//! Denise (MOS 8362 / 8373) Architecture & Video Display Processor
//!
//! Video pixel serializer, bitplane shifters, Dual Playfield, HAM6, EHB,
//! 32-color palette (COLOR00..COLOR31), hardware sprites, and game port counters.

pub use frame_builder;
pub use sprites;

pub use config::DeniseModel;
use config::{stage_mutation, tick_mutations, BeamPosition, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Total number of hardware color palette registers
pub const COLOR_PALETTE_SIZE: usize = 32;

/// Fixed-capacity in-flight register mutation buffer for Denise (covers 32 colors + controls)
pub const DENISE_MUTATION_CAPACITY: usize = 64;

/// Decodes a HAM6 pixel given raw bitplane data and previous held RGB color
#[inline(always)]
pub fn decode_ham6(
    planes_data: u8,
    palette: &[u16; COLOR_PALETTE_SIZE],
    held_rgb: &mut u16,
) -> u16 {
    let ctrl = (planes_data >> 4) & 0x03;
    let data = (planes_data & 0x0F) as u16;

    let r = (*held_rgb >> 8) & 0xF;
    let g = (*held_rgb >> 4) & 0xF;
    let b = *held_rgb & 0xF;

    match ctrl {
        0 => {
            let col = palette[data as usize] & 0x0FFF;
            *held_rgb = col;
            col
        }
        1 => {
            let col = (r << 8) | (g << 4) | data;
            *held_rgb = col;
            col
        }
        2 => {
            let col = (data << 8) | (g << 4) | b;
            *held_rgb = col;
            col
        }
        3 => {
            let col = (r << 8) | (data << 4) | b;
            *held_rgb = col;
            col
        }
        _ => *held_rgb,
    }
}

/// Decodes an Extra Half-Brite (EHB) pixel given raw bitplane data
#[inline(always)]
pub fn decode_ehb(planes_data: u8, palette: &[u16; COLOR_PALETTE_SIZE]) -> u16 {
    let idx = (planes_data & 0x1F) as usize;
    let col = palette[idx];
    if (planes_data & 0x20) != 0 {
        let r = ((col >> 8) & 0xF) >> 1;
        let g = ((col >> 4) & 0xF) >> 1;
        let b = (col & 0xF) >> 1;
        (r << 8) | (g << 4) | b
    } else {
        col
    }
}

/// Decodes a Dual Playfield pixel given raw bitplane data and PF2 priority flag
#[inline(always)]
pub fn decode_dual_playfield(
    planes_data: u8,
    palette: &[u16; COLOR_PALETTE_SIZE],
    pf2_priority: bool,
) -> u16 {
    let pf1_idx = (planes_data & 0x01)
        | (((planes_data >> 2) & 0x01) << 1)
        | (((planes_data >> 4) & 0x01) << 2);

    let pf2_idx = ((planes_data >> 1) & 0x01)
        | (((planes_data >> 3) & 0x01) << 1)
        | (((planes_data >> 5) & 0x01) << 2);

    let pf1_col = if pf1_idx != 0 {
        Some(palette[pf1_idx as usize])
    } else {
        None
    };

    let pf2_col = if pf2_idx != 0 {
        Some(palette[8 + pf2_idx as usize])
    } else {
        None
    };

    if pf2_priority {
        pf2_col.or(pf1_col).unwrap_or(palette[0])
    } else {
        pf1_col.or(pf2_col).unwrap_or(palette[0])
    }
}

/// Denise video display processor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denise {
    /// Active Denise chip hardware model (8362 OCS, 8373 ECS)
    pub model: DeniseModel,
    /// Denise 8 hardware sprite engines
    pub sprites: sprites::Sprites,
    /// Denise raster scanline pixel compositor & frame buffer
    pub frame_builder: frame_builder::FrameBuilder,

    // --- Active Latched Registers (Read is NOW) ---
    /// Bitplane Control 0 (plane count BPU, HIRES, HAM, DBLPF, COLOR)
    pub bplcon0: u16,
    /// Bitplane Control 1 (horizontal scroll offsets for PF1 and PF2)
    pub bplcon1: u16,
    /// Bitplane Control 2 (playfield and sprite priority arbitration)
    pub bplcon2: u16,
    /// Bitplane Control 3 (ECS border color, enhanced sprite control)
    pub bplcon3: u16,
    /// Display Window Start ($08E, upper-left corner: VSTART, HSTART)
    pub diwstrt: u16,
    /// Display Window Stop ($090, lower-right corner: VSTOP, HSTOP)
    pub diwstop: u16,
    /// Display Data Fetch Start ($092)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094)
    pub ddfstop: u16,
    /// Bitplane Modulo 1 ($108, odd bitplanes 1, 3, 5)
    pub bpl1mod: i16,
    /// Bitplane Modulo 2 ($10A, even bitplanes 2, 4, 6)
    pub bpl2mod: i16,
    /// 32 palette color registers (12-bit RGB444: 4 bits R, 4 bits G, 4 bits B)
    pub color: [u16; COLOR_PALETTE_SIZE],
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
    pub bpldat: [u16; 6],
    /// Active parallel bitplane shift registers
    pub shifters: [u16; 6],
    /// Last held color for HAM6 mode
    pub last_ham_rgb: u16,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; DENISE_MUTATION_CAPACITY],
}

impl Denise {
    /// Creates a new Denise instance with the specified hardware model
    pub fn new(model: DeniseModel) -> Self {
        Self {
            model,
            sprites: sprites::Sprites::new(),
            frame_builder: frame_builder::FrameBuilder::new(),
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
            joy0dat: 0,
            joy1dat: 0,
            bpldat: [0; 6],
            shifters: [0; 6],
            last_ham_rgb: 0,
            mutations: [None; DENISE_MUTATION_CAPACITY],
        }
    }

    /// Resets Denise registers to power-on defaults
    pub fn reset(&mut self) {
        self.sprites.reset();
        self.frame_builder.reset();
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
        self.joy0dat = 0;
        self.joy1dat = 0;
        self.bpldat.fill(0);
        self.shifters.fill(0);
        self.last_ham_rgb = 0;
        self.mutations = [None; DENISE_MUTATION_CAPACITY];
    }

    /// Advances Denise pixel pipeline, steps sprites and frame builder,
    /// and processes in-flight mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self, beam: BeamPosition) -> [Option<(u16, u16)>; 8] {
        if beam.hpos == 0 {
            self.last_ham_rgb = self.color[0];
        }

        self.sprites.step_cck(beam);
        self.frame_builder.step_cck(beam);

        // Composite raster pixel into FrameBuilder for the current Color Clock beam position
        let backdrop_argb = frame_builder::rgb444_to_argb32(self.color[0]);
        self.frame_builder
            .set_cck_pixels(beam.hpos, beam.vpos, backdrop_argb);

        let mut due = [None; 8];
        let mut due_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if due_count < due.len() {
                due[due_count] = Some((reg, val));
                due_count += 1;
            }
        });
        for item in due.iter().flatten() {
            self.commit_register_write(item.0, item.1);
        }
        due
    }

    /// Loads 6 parallel bitplane words into active shift registers
    #[inline]
    pub fn load_bitplane_data(&mut self, data: [u16; 6]) {
        self.bpldat = data;
        self.shifters = data;
    }

    /// Shifts out 1 pixel (MSB first) across all 6 bitplanes
    #[inline]
    pub fn shift_pixel(&mut self) -> u8 {
        let mut val = 0u8;
        for (i, shifter) in self.shifters.iter_mut().enumerate() {
            if (*shifter & 0x8000) != 0 {
                val |= 1 << i;
            }
            *shifter <<= 1;
        }
        val
    }

    /// Decodes a 6-bit pixel into 12-bit RGB444 based on active display mode
    pub fn decode_pixel(&mut self, planes_data: u8) -> u16 {
        let plane_count = self.bitplane_count();
        if plane_count == 0 {
            return self.color[0];
        }

        let mask = if plane_count >= 6 {
            0x3F
        } else {
            (1 << plane_count) - 1
        };
        let data = planes_data & mask;

        if self.is_ham() && plane_count == 6 {
            decode_ham6(data, &self.color, &mut self.last_ham_rgb)
        } else if self.is_dual_playfield() {
            let pf2_priority = (self.bplcon2 & 0x0040) != 0;
            decode_dual_playfield(data, &self.color, pf2_priority)
        } else if plane_count == 6 && !self.is_hires() {
            decode_ehb(data, &self.color)
        } else {
            let idx = (data & 0x1F) as usize;
            self.color[idx]
        }
    }

    /// Renders a full horizontal scanline of bitplane word blocks into FrameBuilder
    pub fn render_scanline(&mut self, vpos: u16, word_blocks: &[[u16; 6]]) {
        self.last_ham_rgb = self.color[0];
        let pf1_delay = (self.bplcon1 & 0x0F) as usize;
        let hires = self.is_hires();
        let scale = if hires { 2 } else { 1 };

        let mut pixel_x = 0usize;

        let backdrop_argb = frame_builder::rgb444_to_argb32(self.color[0]);
        for _ in 0..(pf1_delay * scale) {
            self.frame_builder
                .set_pixel(pixel_x, vpos as usize, backdrop_argb);
            pixel_x += 1;
        }

        for block in word_blocks {
            self.load_bitplane_data(*block);

            for _ in 0..16 {
                let pixel_data = self.shift_pixel();
                let rgb = self.decode_pixel(pixel_data);
                let argb = frame_builder::rgb444_to_argb32(rgb);

                for _ in 0..scale {
                    self.frame_builder.set_pixel(pixel_x, vpos as usize, argb);
                    pixel_x += 1;
                }
            }
        }
    }

    /// Action method: sets BPLCON0 and updates active display mode flags
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
    }

    /// Action method: sets BPLCON1 horizontal scroll offsets
    #[inline]
    pub fn set_bplcon1(&mut self, val: u16) {
        self.bplcon1 = val;
    }

    /// Action method: sets BPLCON2 priority flags
    #[inline]
    pub fn set_bplcon2(&mut self, val: u16) {
        self.bplcon2 = val;
    }

    /// Action method: sets an individual RGB444 color palette entry
    #[inline]
    pub fn set_color(&mut self, index: usize, rgb: u16) {
        if index < COLOR_PALETTE_SIZE {
            self.color[index] = rgb & 0x0FFF;
        }
    }

    /// Action method: sets display window clipping coordinates
    #[inline]
    pub fn set_diw(&mut self, strt: u16, stop: u16) {
        self.diwstrt = strt;
        self.diwstop = stop;
    }

    /// Returns the active number of bitplanes (0..6)
    #[inline]
    pub fn bitplane_count(&self) -> u8 {
        ((self.bplcon0 >> 12) & 0x07) as u8
    }

    /// Returns true if High-Resolution (640-pixel) mode is enabled
    #[inline]
    pub fn is_hires(&self) -> bool {
        (self.bplcon0 & 0x8000) != 0
    }

    /// Returns true if Hold-And-Modify (HAM) mode is enabled
    #[inline]
    pub fn is_ham(&self) -> bool {
        (self.bplcon0 & 0x0800) != 0
    }

    /// Returns true if Dual Playfield mode is enabled
    #[inline]
    pub fn is_dual_playfield(&self) -> bool {
        (self.bplcon0 & 0x0400) != 0
    }

    /// Reads collision data register (CLXDAT at $DFF00E) and clears it immediately on read
    #[inline]
    pub fn read_clxdat(&mut self) -> u16 {
        let val = self.clxdat;
        self.clxdat = 0;
        val
    }

    /// Reads register with physical read side-effects (e.g. clearing CLXDAT).
    /// Write-only registers return floating open bus $FFFF.
    pub fn read_register(&mut self, offset: u16) -> u16 {
        match offset & 0x1FE {
            0x00A => self.joy0dat,
            0x00C => self.joy1dat,
            0x00E => self.read_clxdat(),
            _ => 0xFFFF,
        }
    }

    /// Read-only inspection without side-effects (for debuggers and GUI)
    #[inline]
    pub fn peek_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            0x00A => self.joy0dat,
            0x00C => self.joy1dat,
            0x00E => self.clxdat,
            _ => 0xFFFF,
        }
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode.
    /// Returns `Some((offset, val))` if committed immediately, or `None` if staged in pipeline.
    pub fn write_register(&mut self, offset: u16, val: u16) -> Option<(u16, u16)> {
        let offset = offset & 0x1FE;
        let (delay, mode) = match offset {
            0x100 | 0x102 | 0x104 | 0x106 => (1, MutationMode::OverwritePending), // BPLCON0..3
            0x098 => (1, MutationMode::OverwritePending),                         // CLXCON
            0x08E | 0x090 => (1, MutationMode::OverwritePending), // DIWSTRT, DIWSTOP
            0x180..=0x1BE => (1, MutationMode::Pipeline),         // COLOR00..31
            0x110..=0x11A => (1, MutationMode::Pipeline),         // BPL1DAT..6DAT
            0x140..=0x17E => (1, MutationMode::OverwritePending), // SPRx
            _ => (1, MutationMode::OverwritePending),
        };

        if !stage_mutation(&mut self.mutations, offset, val, delay, mode) {
            self.commit_register_write(offset, val);
            Some((offset, val))
        } else {
            None
        }
    }

    /// Commits a register write directly into active Denise silicon state
    pub fn commit_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            0x100 => self.set_bplcon0(val),
            0x102 => self.set_bplcon1(val),
            0x104 => self.set_bplcon2(val),
            0x106 => self.bplcon3 = val,
            0x08E => self.diwstrt = val,
            0x090 => self.diwstop = val,
            0x098 => self.clxcon = val,
            0x034 => self.potgo = val,

            // Color palette $180..$1BE
            0x180..=0x1BE => {
                let idx = ((offset - 0x180) / 2) as usize;
                if idx < COLOR_PALETTE_SIZE {
                    self.color[idx] = val & 0x0FFF;
                }
            }

            // Bitplane parallel data latches
            0x110..=0x11A => {
                let plane = ((offset - 0x110) / 2) as usize;
                if plane < 6 {
                    self.bpldat[plane] = val;
                }
            }

            // Sprite registers
            0x140..=0x17E => {
                let spr = ((offset - 0x140) / 8) as usize;
                let sub = ((offset - 0x140) % 8) / 2;
                if spr < 8 {
                    match sub {
                        0 => self.sprites.channels[spr].pos = val,
                        1 => self.sprites.channels[spr].ctl = val,
                        2 => self.sprites.channels[spr].data_a = val,
                        3 => self.sprites.channels[spr].data_b = val,
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    /// Writes to color palette register directly (COLOR00..COLOR31 at $DFF180..$DFF1BE)
    #[inline]
    pub fn write_color(&mut self, index: usize, val: u16) {
        if index < COLOR_PALETTE_SIZE {
            self.color[index] = val & 0x0FFF;
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
