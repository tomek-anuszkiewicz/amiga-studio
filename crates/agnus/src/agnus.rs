//! Agnus (MOS 8370 / 8371 / 8372A) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub use blitter;
pub use copper;
pub use dma;

pub use config::AgnusModel;
use config::{stage_mutation, tick_mutations, BeamPosition, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Maximum horizontal Color Clock cycles per scanline (PAL)
pub const PAL_LINE_CCKS: u16 = 227;
/// Total vertical scanlines per frame (PAL)
pub const PAL_FRAME_LINES: u16 = 312;
/// Maximum horizontal Color Clock cycles per scanline (NTSC)
pub const NTSC_LINE_CCKS: u16 = 227;
/// Total vertical scanlines per frame (NTSC)
pub const NTSC_FRAME_LINES: u16 = 262;

/// Fixed-capacity in-flight register mutation buffer for Agnus (covers all writable registers)
pub const AGNUS_MUTATION_CAPACITY: usize = 64;

/// Agnus custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agnus {
    /// Active Agnus chip hardware model (8370, 8371, 8372A)
    pub model: AgnusModel,
    /// Agnus Copper coprocessor
    pub copper: copper::Copper,
    /// Agnus 4-channel DMA Blitter
    pub blitter: blitter::Blitter,
    /// Agnus DMA slot arbiter & scheduler
    pub dma: dma::DmaScheduler,
    /// Horizontal raster beam counter (0..227 CCK)
    pub hpos: u16,
    /// Vertical scanline counter (0..312 PAL, 0..262 NTSC)
    pub vpos: u16,
    /// Long Frame toggle bit (toggled every field in interlace mode)
    pub lof: bool,
    /// True if Chip RAM is currently blocked by custom chip DMA
    pub chip_ram_blocked: bool,

    // --- Active Latched Registers (Read is NOW) ---
    /// DMACON / DMACONR ($096 / $002) - active DMA channel enables
    pub dmacon: u16,
    /// COPCON ($02E) - Copper control register (bit 1: CDANG)
    pub copcon: u16,
    /// Copper location pointer 1 ($080-$082)
    pub cop1lc: u32,
    /// Copper location pointer 2 ($084-$086)
    pub cop2lc: u32,
    /// Display Window Start ($08E)
    pub diwstrt: u16,
    /// Display Window Stop ($090)
    pub diwstop: u16,
    /// Display Data Fetch Start ($092)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094)
    pub ddfstop: u16,
    /// Bitplane Control 0 ($100, latched by Agnus for DMA slot count)
    pub bplcon0: u16,
    /// Bitplane Control 1 ($102)
    pub bplcon1: u16,
    /// Bitplane Modulo 1 ($108, odd bitplanes)
    pub bpl1mod: i16,
    /// Bitplane Modulo 2 ($10A, even bitplanes)
    pub bpl2mod: i16,
    /// Bitplane DMA pointers 1..6 ($0E0-$0F6)
    pub bplpt: [u32; 6],
    /// Sprite DMA pointers 0..7 ($120-$13E)
    pub sprpt: [u32; 8],
    /// Audio DMA pointers 0..3 ($0A0, $0B0, $0C0, $0D0)
    pub audpt: [u32; 4],
    /// Floppy Disk DMA pointer ($020-$022)
    pub dskpt: u32,

    // --- Blitter Active Registers ($040-$074) ---
    pub bltcon0: u16,
    pub bltcon1: u16,
    pub bltafwm: u16,
    pub bltalwm: u16,
    pub bltcpt: u32,
    pub bltbpt: u32,
    pub bltapt: u32,
    pub bltdpt: u32,
    pub bltsize: u16,
    pub bltcmod: i16,
    pub bltbmod: i16,
    pub bltamod: i16,
    pub bltdmod: i16,
    pub bltcdat: u16,
    pub bltbdat: u16,
    pub bltadat: u16,
    pub blitter_busy: bool,
    pub blitter_zero: bool,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; AGNUS_MUTATION_CAPACITY],
}

impl Agnus {
    /// Creates a new Agnus instance with specified chip revision
    pub fn new(model: AgnusModel) -> Self {
        Self {
            model,
            copper: copper::Copper::new(),
            blitter: blitter::Blitter::new(),
            dma: dma::DmaScheduler::new(),
            hpos: 0,
            vpos: 0,
            lof: false,
            chip_ram_blocked: false,
            dmacon: 0,
            copcon: 0,
            cop1lc: 0,
            cop2lc: 0,
            diwstrt: 0,
            diwstop: 0,
            ddfstrt: 0x0038,
            ddfstop: 0x00D0,
            bplcon0: 0,
            bplcon1: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            bplpt: [0; 6],
            sprpt: [0; 8],
            audpt: [0; 4],
            dskpt: 0,
            bltcon0: 0,
            bltcon1: 0,
            bltafwm: 0xFFFF,
            bltalwm: 0xFFFF,
            bltcpt: 0,
            bltbpt: 0,
            bltapt: 0,
            bltdpt: 0,
            bltsize: 0,
            bltcmod: 0,
            bltbmod: 0,
            bltamod: 0,
            bltdmod: 0,
            bltcdat: 0,
            bltbdat: 0,
            bltadat: 0,
            blitter_busy: false,
            blitter_zero: false,
            mutations: [None; AGNUS_MUTATION_CAPACITY],
        }
    }

    /// Resets Agnus registers and beam counters to power-on defaults
    pub fn reset(&mut self) {
        self.copper.reset();
        self.blitter.reset();
        self.dma.reset();
        self.hpos = 0;
        self.vpos = 0;
        self.lof = false;
        self.chip_ram_blocked = false;
        self.dmacon = 0;
        self.copcon = 0;
        self.cop1lc = 0;
        self.cop2lc = 0;
        self.diwstrt = 0;
        self.diwstop = 0;
        self.ddfstrt = 0x0038;
        self.ddfstop = 0x00D0;
        self.bplcon0 = 0;
        self.bplcon1 = 0;
        self.bpl1mod = 0;
        self.bpl2mod = 0;
        self.bplpt.fill(0);
        self.sprpt.fill(0);
        self.audpt.fill(0);
        self.dskpt = 0;
        self.bltcon0 = 0;
        self.bltcon1 = 0;
        self.bltafwm = 0xFFFF;
        self.bltalwm = 0xFFFF;
        self.bltcpt = 0;
        self.bltbpt = 0;
        self.bltapt = 0;
        self.bltdpt = 0;
        self.bltsize = 0;
        self.bltcmod = 0;
        self.bltbmod = 0;
        self.bltamod = 0;
        self.bltdmod = 0;
        self.bltcdat = 0;
        self.bltbdat = 0;
        self.bltadat = 0;
        self.blitter_busy = false;
        self.blitter_zero = false;
        self.mutations = [None; AGNUS_MUTATION_CAPACITY];
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; 8] {
        // 1. Advance horizontal and vertical raster beam counters
        let max_lines = match self.model {
            AgnusModel::OcsNtsc8370 => NTSC_FRAME_LINES,
            _ => PAL_FRAME_LINES,
        };

        self.hpos = self.hpos.wrapping_add(1);
        if self.hpos > PAL_LINE_CCKS {
            self.hpos = 0;
            self.vpos = self.vpos.wrapping_add(1);
            if self.vpos >= max_lines {
                self.vpos = 0;
                self.lof = !self.lof;
            }
        }

        // 2. Step embedded coprocessors and schedulers
        let beam = self.beam();
        self.copper.step_cck(beam);
        self.blitter.step_cck();
        self.dma.step_cck();
        self.chip_ram_blocked = self
            .dma
            .is_chip_ram_blocked(beam.hpos, self.blitter.is_busy);

        // 3. Process and commit due register mutations
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

    /// Returns the current raster beam position snapshot
    #[inline]
    pub fn beam(&self) -> BeamPosition {
        BeamPosition::new(self.hpos, self.vpos, self.lof)
    }

    /// Returns true if Chip RAM is currently locked by custom chip DMA
    #[inline]
    pub fn is_chip_ram_blocked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Reads Vertical & Horizontal beam position (VHPOSR at $DFF006)
    #[inline]
    pub fn vhposr(&self) -> u16 {
        let v_low = (self.vpos & 0xFF) as u16;
        let h = (self.hpos & 0xFF) as u16;
        (v_low << 8) | h
    }

    /// Reads Vertical beam position high bit, chip ID, and LOF (VPOSR at $DFF004)
    #[inline]
    pub fn vposr(&self) -> u16 {
        let mut val = 0u16;
        if self.lof {
            val |= 0x8000;
        }
        match self.model {
            AgnusModel::OcsNtsc8370 => val |= 0x1000,
            AgnusModel::OcsPal8371 => {}
        }
        val |= (self.vpos >> 8) & 0x07;
        val
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline]
    pub fn read_dmaconr(&self) -> u16 {
        let mut val = self.dmacon & 0x07FF;
        if self.blitter_busy || self.blitter.is_busy {
            val |= 0x8000; // BBUSY
        }
        if self.blitter_zero || self.blitter.is_zero {
            val |= 0x4000; // BZERO
        }
        val
    }

    /// Reads an Agnus register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            0x002 => self.read_dmaconr(),
            0x004 => self.vposr(),
            0x006 => self.vhposr(),
            _ => 0xFFFF,
        }
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode.
    /// Returns `Some((offset, val))` if the register was committed immediately (e.g. overflow fallback or delay 0),
    /// or `None` if staged in the mutation pipeline.
    pub fn write_register(&mut self, offset: u16, val: u16) -> Option<(u16, u16)> {
        let offset = offset & 0x1FE;
        let (delay, mode) = match offset {
            0x096 => (2, MutationMode::OverwritePending), // DMACON
            0x088 | 0x08A => (1, MutationMode::OverwritePending), // COPJMP1, COPJMP2
            0x058 => (1, MutationMode::OverwritePending), // BLTSIZE
            0x100 => (4, MutationMode::OverwritePending), // BPLCON0 (Agnus DMA allocation)
            0x08E | 0x090 => (4, MutationMode::OverwritePending), // DIWSTRT, DIWSTOP
            0x092 | 0x094 => (4, MutationMode::OverwritePending), // DDFSTRT, DDFSTOP
            0x108 | 0x10A => (2, MutationMode::OverwritePending), // BPL1MOD, BPL2MOD
            0x040..=0x056 => (2, MutationMode::OverwritePending), // BLTCON0..BLTDPTH/L
            0x060..=0x066 => (2, MutationMode::OverwritePending), // BLTCMOD..BLTDMOD
            0x070..=0x074 => (1, MutationMode::Pipeline), // BLTCDAT..BLTADAT
            0x080..=0x086 => (2, MutationMode::OverwritePending), // COP1LC, COP2LC
            0x0E0..=0x0F6 => (2, MutationMode::OverwritePending), // BPLxPTH/L
            0x120..=0x13E => (2, MutationMode::OverwritePending), // SPRxPTH/L
            _ => (2, MutationMode::OverwritePending),
        };

        if !stage_mutation(&mut self.mutations, offset, val, delay, mode) {
            self.commit_register_write(offset, val);
            Some((offset, val))
        } else {
            None
        }
    }

    /// Commits a register write directly into active Agnus silicon state
    pub fn commit_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            0x096 => {
                // DMACON SET/CLR logic
                if (val & 0x8000) != 0 {
                    self.dmacon |= val & 0x7FFF;
                } else {
                    self.dmacon &= !(val & 0x7FFF);
                }
            }
            0x02E => self.copcon = val,
            0x080 => self.cop1lc = (self.cop1lc & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x082 => self.cop1lc = (self.cop1lc & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x084 => self.cop2lc = (self.cop2lc & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x086 => self.cop2lc = (self.cop2lc & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x08E => self.diwstrt = val,
            0x090 => self.diwstop = val,
            0x092 => self.ddfstrt = val & 0x00FC,
            0x094 => self.ddfstop = val & 0x00FC,
            0x100 => self.bplcon0 = val,
            0x102 => self.bplcon1 = val,
            0x108 => self.bpl1mod = val as i16,
            0x10A => self.bpl2mod = val as i16,

            // Blitter registers
            0x040 => self.bltcon0 = val,
            0x042 => self.bltcon1 = val,
            0x044 => self.bltafwm = val,
            0x046 => self.bltalwm = val,
            0x048 => self.bltcpt = (self.bltcpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x04A => self.bltcpt = (self.bltcpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x04C => self.bltbpt = (self.bltbpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x04E => self.bltbpt = (self.bltbpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x050 => self.bltapt = (self.bltapt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x052 => self.bltapt = (self.bltapt & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x054 => self.bltdpt = (self.bltdpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16),
            0x056 => self.bltdpt = (self.bltdpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32),
            0x058 => {
                self.bltsize = val;
                self.blitter_busy = true;
            }
            0x060 => self.bltcmod = val as i16,
            0x062 => self.bltbmod = val as i16,
            0x064 => self.bltamod = val as i16,
            0x066 => self.bltdmod = val as i16,
            0x070 => self.bltcdat = val,
            0x072 => self.bltbdat = val,
            0x074 => self.bltadat = val,

            // Bitplane pointers
            0x0E0..=0x0F6 => {
                let plane = ((offset - 0x0E0) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if plane < 6 {
                    if is_low {
                        self.bplpt[plane] =
                            (self.bplpt[plane] & 0xFFFF_0000) | ((val & 0xFFFE) as u32);
                    } else {
                        self.bplpt[plane] =
                            (self.bplpt[plane] & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16);
                    }
                }
            }

            // Sprite pointers
            0x120..=0x13E => {
                let sprite = ((offset - 0x120) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if sprite < 8 {
                    if is_low {
                        self.sprpt[sprite] =
                            (self.sprpt[sprite] & 0xFFFF_0000) | ((val & 0xFFFE) as u32);
                    } else {
                        self.sprpt[sprite] =
                            (self.sprpt[sprite] & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16);
                    }
                }
            }

            _ => {}
        }
    }

    /// Queries whether a specific DMA channel is enabled in DMACON
    #[inline]
    pub fn is_dma_enabled(&self, mask: u16) -> bool {
        // Master DMAEN (bit 9) must be set
        (self.dmacon & 0x0200) != 0 && (self.dmacon & mask) != 0
    }

    /// Returns true if Blitter Nasty (BLTPRI, bit 10) is enabled
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & 0x0400) != 0
    }
}
