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
        self.mutations = [None; DENISE_MUTATION_CAPACITY];
    }

    /// Advances Denise pixel pipeline, steps sprites and frame builder,
    /// and processes in-flight mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self, beam: BeamPosition) -> [Option<(u16, u16)>; 8] {
        self.sprites.step_cck(beam);
        self.frame_builder.step_cck(beam);

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
