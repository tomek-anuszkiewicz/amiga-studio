//! Agnus (MOS 8370 / 8371 / 8372A) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub use blitter;
pub use copper;
pub use dma;

use config::custom_reg;
use config::mask::dmacon;
pub use config::AgnusModel;
use config::{stage_mutation, tick_mutations, BeamPosition, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Maximum horizontal Color Clock cycles per scanline (PAL)
pub const PAL_LINE_CCKS: u16 = 227;
/// Total vertical scanlines per frame (PAL)
pub const PAL_FRAME_LINES: u16 = 312;
/// Maximum horizontal Color Clock cycles per scanline (NTSC short line)
pub const NTSC_LINE_CCKS: u16 = 227;
/// Short scanline length in Color Clocks (NTSC standard)
pub const NTSC_SHORT_LINE_CCKS: u16 = 227;
/// Long scanline length in Color Clocks (NTSC interlace LOL bit active)
pub const NTSC_LONG_LINE_CCKS: u16 = 228;
/// Total vertical scanlines per frame (NTSC)
pub const NTSC_FRAME_LINES: u16 = 262;

/// Number of Color Clocks that Agnus's internal scheduling counter leads the visible display beam
pub const VHPOSR_PIPELINE_LEAD_CCKS: u16 = 5;
/// Number of Color Clocks after line wrap during which the vertical ripple counter is settling
pub const VHPOSR_VERTICAL_SETTLE_CCKS: u16 = 1;

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
    /// Long Line toggle bit for PAL (alternates between 227 and 228 CCKs per scanline)
    pub lol: bool,
    /// True if Chip RAM is currently blocked by custom chip DMA
    pub chip_ram_blocked: bool,
    /// Pending bitplane DMA word fetched from Chip RAM to be routed to Denise BPLxDAT
    pub pending_bpl_dma: Option<(u8, u16)>,

    // --- Active Latched Registers (Read is NOW) ---
    /// DMACON / DMACONR ($096 / $002) - active DMA channel enables
    pub dmacon: u16,
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
    /// Audio DMA start/loop locations 0..3 (AUDxLCH/AUDxLCL)
    pub audlc: [u32; 4],
    /// Floppy Disk DMA pointer ($020-$022)
    pub dskpt: u32,
    /// Vertical blanking interrupt request strobe
    #[serde(default)]
    pub vblank_irq: bool,
    /// Pending custom register write emitted by the Copper on this cycle
    #[serde(default)]
    pub pending_copper_write: Option<(u16, u16)>,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; AGNUS_MUTATION_CAPACITY],
}

impl Agnus {
    /// Creates a new Agnus instance with specified chip revision
    pub fn new(model: AgnusModel) -> Self {
        Self {
            model,
            vblank_irq: false,
            pending_copper_write: None,
            copper: copper::Copper::new(),
            blitter: blitter::Blitter::new(),
            dma: dma::DmaScheduler::new(),
            hpos: 0,
            vpos: 0,
            lof: false,
            lol: false,
            chip_ram_blocked: false,
            pending_bpl_dma: None,
            dmacon: 0,
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
            audlc: [0; 4],
            dskpt: 0,
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
        self.lol = false;
        self.chip_ram_blocked = false;
        self.pending_bpl_dma = None;
        self.dmacon = 0;
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
        self.audlc.fill(0);
        self.dskpt = 0;
        self.vblank_irq = false;
        self.pending_copper_write = None;
        self.mutations = [None; AGNUS_MUTATION_CAPACITY];
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    #[inline]
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; 8] {
        self.step_cck_ram(&mut [])
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers with Chip RAM access,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck_ram(&mut self, chip_ram: &mut [u8]) -> [Option<(u16, u16)>; 8] {
        // 1. Advance horizontal and vertical raster beam counters
        let max_lines = match self.model {
            AgnusModel::OcsNtsc8370 => NTSC_FRAME_LINES,
            _ => PAL_FRAME_LINES,
        };
        let line_ccks = match self.model {
            AgnusModel::OcsNtsc8370 => {
                if self.lol {
                    228
                } else {
                    227
                }
            }
            _ => PAL_LINE_CCKS,
        };

        self.hpos = self.hpos.wrapping_add(1);
        if self.hpos >= line_ccks {
            self.hpos = 0;
            if matches!(self.model, AgnusModel::OcsNtsc8370) {
                self.lol = !self.lol;
            }
            self.vpos = self.vpos.wrapping_add(1);
            if self.vpos >= max_lines {
                self.vpos = 0;
                self.lof = !self.lof;
                self.vblank_irq = true;
            }
        }

        // 2. Step embedded coprocessors and schedulers
        let beam = self.beam();
        self.pending_copper_write = self.copper.step_cck(beam, self.blitter.is_busy, chip_ram);
        self.dma.step_cck();

        // Evaluate 8-tier Master DMA Bus Arbitration
        let copper_wants_bus = self.copper.is_active_fetching();
        let blitter_wants_bus = self.blitter.is_busy;
        let owner = self.dma.arbitrate(
            beam.hpos,
            beam.vpos,
            self.dskpt != 0,
            [true, true, true, true],
            copper_wants_bus,
            blitter_wants_bus,
            true,
        );
        self.chip_ram_blocked = self.dma.chip_ram_blocked;

        match owner {
            dma::DmaChannel::Blitter => {
                self.blitter.step_cck_ram(chip_ram);
            }
            dma::DmaChannel::Bitplane(plane) => {
                let p = plane as usize;
                if p < 6 && !chip_ram.is_empty() {
                    let addr = (self.bplpt[p] as usize) & (chip_ram.len().wrapping_sub(1));
                    if addr + 1 < chip_ram.len() {
                        let word = u16::from_be_bytes([chip_ram[addr], chip_ram[addr + 1]]);
                        self.pending_bpl_dma = Some((plane, word));
                        let mod_inc = if self.dma.is_last_bpl_block(self.hpos) {
                            if p % 2 == 0 {
                                self.bpl1mod as i32
                            } else {
                                self.bpl2mod as i32
                            }
                        } else {
                            0
                        };
                        self.bplpt[p] = self.bplpt[p].wrapping_add(2).wrapping_add(mod_inc as u32)
                            & 0x0007_FFFE;
                    }
                }
            }
            dma::DmaChannel::Sprite(sprite) => {
                let s = sprite as usize;
                if s < 8 && !chip_ram.is_empty() {
                    let addr = (self.sprpt[s] as usize) & (chip_ram.len().wrapping_sub(1));
                    if addr + 1 < chip_ram.len() {
                        self.sprpt[s] = self.sprpt[s].wrapping_add(2) & 0x0007_FFFE;
                    }
                }
            }
            dma::DmaChannel::Audio(ch) => {
                let c = ch as usize;
                if c < 4 && !chip_ram.is_empty() {
                    let addr = (self.audpt[c] as usize) & (chip_ram.len().wrapping_sub(1));
                    if addr + 1 < chip_ram.len() {
                        self.audpt[c] = self.audpt[c].wrapping_add(2) & 0x0007_FFFE;
                    }
                }
            }
            _ => {}
        }

        // 3. Process and commit due register mutations
        let mut committed = [None; AGNUS_MUTATION_CAPACITY];
        let mut committed_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if committed_count < committed.len() {
                committed[committed_count] = Some((reg, val));
                committed_count += 1;
            }
        });
        for item in committed[..committed_count].iter().flatten() {
            self.commit_register_write(item.0, item.1);
        }

        let mut due = [None; 8];
        for (i, item) in committed[..committed_count.min(8)].iter().enumerate() {
            due[i] = *item;
        }
        due
    }

    /// Polls and clears any custom register write emitted by the Copper on this cycle
    #[inline]
    pub fn poll_copper_write(&mut self) -> Option<(u16, u16)> {
        self.pending_copper_write.take()
    }

    /// Polls and clears any bitplane DMA word fetched from Chip RAM on this cycle
    #[inline]
    pub fn poll_bpl_dma(&mut self) -> Option<(u8, u16)> {
        self.pending_bpl_dma.take()
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

    /// Returns the pipelined (H, V) beam position as seen by CPU register reads (5 CCKs ahead of display beam).
    #[inline]
    fn pipelined_beam_readout(&self) -> (u16, u16) {
        let max_lines = match self.model {
            AgnusModel::OcsNtsc8370 => NTSC_FRAME_LINES,
            _ => PAL_FRAME_LINES,
        };
        let line_ccks = match self.model {
            AgnusModel::OcsNtsc8370 => {
                if self.lol {
                    NTSC_LONG_LINE_CCKS
                } else {
                    NTSC_SHORT_LINE_CCKS
                }
            }
            _ => PAL_LINE_CCKS,
        };

        let mut h = self.hpos + VHPOSR_PIPELINE_LEAD_CCKS;
        let mut v = self.vpos;
        if h >= line_ccks {
            h -= line_ccks;
            v = v.wrapping_add(1);
            if v >= max_lines {
                v = 0;
            }
        }

        let effective_v = if h <= VHPOSR_VERTICAL_SETTLE_CCKS {
            self.vpos
        } else {
            v
        };
        (h as u16, effective_v)
    }

    /// Reads Vertical & Horizontal beam position (VHPOSR at $DFF006)
    ///
    /// The returned beam position reflects internal Agnus pipeline and bus latency (~5 CCKs ahead of display beam).
    #[inline]
    pub fn vhposr(&self) -> u16 {
        let (h, effective_v) = self.pipelined_beam_readout();
        let v_low = (effective_v & 0xFF) as u16;
        (v_low << 8) | (h & 0xFF)
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

        let (_h, effective_v) = self.pipelined_beam_readout();
        val |= (effective_v >> 8) & 0x07;
        val
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline]
    pub fn read_dmaconr(&self) -> u16 {
        let mut val = self.dmacon & 0x07FF;
        if self.blitter.is_busy {
            val |= dmacon::BBUSY;
        }
        if self.blitter.is_zero {
            val |= dmacon::BZERO;
        }
        val
    }

    /// Reads an Agnus register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            custom_reg::DMACONR => self.read_dmaconr(),
            custom_reg::VPOSR => self.vposr(),
            custom_reg::VHPOSR => self.vhposr(),
            _ => 0xFFFF,
        }
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode.
    /// Returns `Some((offset, val))` if the register was committed immediately (e.g. overflow fallback or delay 0),
    /// or `None` if staged in the mutation pipeline.
    pub fn write_register(&mut self, offset: u16, val: u16) -> Option<(u16, u16)> {
        let offset = offset & 0x1FE;
        let (delay, mode) = match offset {
            custom_reg::DMACON => (2, MutationMode::OverwritePending),
            custom_reg::COPJMP1 | custom_reg::COPJMP2 => (1, MutationMode::OverwritePending),
            custom_reg::BLTSIZE => (1, MutationMode::OverwritePending),
            custom_reg::BPLCON0 => (4, MutationMode::OverwritePending), // BPLCON0 (Agnus DMA allocation)
            custom_reg::DIWSTRT | custom_reg::DIWSTOP => (4, MutationMode::OverwritePending),
            custom_reg::DDFSTRT | custom_reg::DDFSTOP => (4, MutationMode::OverwritePending),
            custom_reg::BPL1MOD | custom_reg::BPL2MOD => (2, MutationMode::OverwritePending),
            custom_reg::BLTCON0..=custom_reg::BLTDPTL => (2, MutationMode::OverwritePending),
            custom_reg::BLTCMOD..=custom_reg::BLTDMOD => (2, MutationMode::OverwritePending),
            custom_reg::BLTCDAT..=custom_reg::BLTADAT => (1, MutationMode::Pipeline),
            custom_reg::COP1LCH..=custom_reg::COP2LCL => (2, MutationMode::OverwritePending),
            custom_reg::BPL1PTH..=custom_reg::BPL6PTL => (2, MutationMode::OverwritePending),
            custom_reg::SPR0PTH..=custom_reg::SPR7PTL => (2, MutationMode::OverwritePending),
            custom_reg::DSKPTH | custom_reg::DSKPTL => (2, MutationMode::OverwritePending),
            custom_reg::AUD0LCH
            | custom_reg::AUD0LCL
            | custom_reg::AUD1LCH
            | custom_reg::AUD1LCL
            | custom_reg::AUD2LCH
            | custom_reg::AUD2LCL
            | custom_reg::AUD3LCH
            | custom_reg::AUD3LCL => (2, MutationMode::OverwritePending),
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
            custom_reg::DMACON => {
                // DMACON SET/CLR logic
                if (val & dmacon::SET_CLR) != 0 {
                    self.dmacon |= val & 0x7FFF;
                } else {
                    self.dmacon &= !(val & 0x7FFF);
                }
                self.dma.write_dmacon(val);
                let dma_en = self.is_dma_enabled(dmacon::BLTEN);
                self.blitter.set_dma_enabled(dma_en);
                self.blitter.set_bltpri(self.is_blitter_nasty());
                self.copper
                    .set_dma_enabled(self.is_dma_enabled(dmacon::COPEN));
            }
            custom_reg::COPCON => self.copper.set_copcon(val),
            custom_reg::COP1LCH => {
                self.copper.cop1lc =
                    (self.copper.cop1lc & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::COP1LCL => {
                self.copper.cop1lc = (self.copper.cop1lc & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::COP2LCH => {
                self.copper.cop2lc =
                    (self.copper.cop2lc & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::COP2LCL => {
                self.copper.cop2lc = (self.copper.cop2lc & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::COPJMP1 => self.copper.restart_list1(),
            custom_reg::COPJMP2 => self.copper.restart_list2(),
            custom_reg::DIWSTRT => {
                self.diwstrt = val;
                self.dma.set_diwstrt(val);
            }
            custom_reg::DIWSTOP => {
                self.diwstop = val;
                self.dma.set_diwstop(val);
            }
            custom_reg::DDFSTRT => {
                self.ddfstrt = val & 0x00FC;
                self.dma.set_ddfstrt(val & 0x00FC);
            }
            custom_reg::DDFSTOP => {
                self.ddfstop = val & 0x00FC;
                self.dma.set_ddfstop(val & 0x00FC);
            }
            custom_reg::BPLCON0 => self.set_bplcon0(val),
            custom_reg::BPLCON1 => self.bplcon1 = val,
            custom_reg::BPL1MOD => self.bpl1mod = val as i16,
            custom_reg::BPL2MOD => self.bpl2mod = val as i16,

            // Blitter registers
            custom_reg::BLTCON0 => self.blitter.bltcon0 = val,
            custom_reg::BLTCON1 => self.blitter.bltcon1 = val,
            custom_reg::BLTAFWM => self.blitter.bltafwm = val,
            custom_reg::BLTALWM => self.blitter.bltalwm = val,
            custom_reg::BLTCPTH => {
                self.blitter.bltcpt =
                    (self.blitter.bltcpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::BLTCPTL => {
                self.blitter.bltcpt = (self.blitter.bltcpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::BLTBPTH => {
                self.blitter.bltbpt =
                    (self.blitter.bltbpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::BLTBPTL => {
                self.blitter.bltbpt = (self.blitter.bltbpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::BLTAPTH => {
                self.blitter.bltapt =
                    (self.blitter.bltapt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::BLTAPTL => {
                self.blitter.bltapt = (self.blitter.bltapt & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::BLTDPTH => {
                self.blitter.bltdpt =
                    (self.blitter.bltdpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16)
            }
            custom_reg::BLTDPTL => {
                self.blitter.bltdpt = (self.blitter.bltdpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32)
            }
            custom_reg::BLTSIZE => self.blitter.trigger_blit(val),
            custom_reg::BLTCMOD => self.blitter.bltcmod = val as i16,
            custom_reg::BLTBMOD => self.blitter.bltbmod = val as i16,
            custom_reg::BLTAMOD => self.blitter.bltamod = val as i16,
            custom_reg::BLTDMOD => self.blitter.bltdmod = val as i16,
            custom_reg::BLTCDAT => self.blitter.bltcdat = val,
            custom_reg::BLTBDAT => self.blitter.bltbdat = val,
            custom_reg::BLTADAT => self.blitter.bltadat = val,

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

            // Floppy Disk DMA pointer
            custom_reg::DSKPTH => {
                self.dskpt = (self.dskpt & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16);
            }
            custom_reg::DSKPTL => {
                self.dskpt = (self.dskpt & 0xFFFF_0000) | ((val & 0xFFFE) as u32);
            }

            // Audio DMA location pointers (AUD0LCH/LCL..AUD3LCH/LCL)
            0x0A0 | 0x0A2 | 0x0B0 | 0x0B2 | 0x0C0 | 0x0C2 | 0x0D0 | 0x0D2 => {
                let ch = ((offset - 0x0A0) / 0x10) as usize;
                if ch < 4 {
                    if (offset & 2) == 0 {
                        self.audlc[ch] =
                            (self.audlc[ch] & 0x0000_FFFF) | (((val & 0x001F) as u32) << 16);
                    } else {
                        self.audlc[ch] = (self.audlc[ch] & 0xFFFF_0000) | ((val & 0xFFFE) as u32);
                    }
                    self.audpt[ch] = self.audlc[ch];
                }
            }

            _ => {}
        }
    }

    /// Action method: sets BPLCON0 and synchronizes Agnus DMA scheduler
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
        self.dma.set_bplcon0(val);
    }

    /// Reloads audio DMA pointer from latched start address (AUDxLC) for channel `channel` (0..3)
    #[inline]
    pub fn reload_audio_ptr(&mut self, channel: usize) {
        if channel < 4 {
            self.audpt[channel] = self.audlc[channel];
        }
    }

    /// Polls and clears the blitter completion interrupt flag
    #[inline]
    pub fn poll_blitter_irq(&mut self) -> bool {
        self.blitter.poll_blit_irq()
    }

    /// Polls and clears the vertical blanking interval interrupt request flag
    #[inline]
    pub fn poll_vblank_irq(&mut self) -> bool {
        let pending = self.vblank_irq;
        self.vblank_irq = false;
        pending
    }

    /// Queries whether a specific DMA channel is enabled in DMACON
    #[inline]
    pub fn is_dma_enabled(&self, mask: u16) -> bool {
        // Master DMAEN (bit 9) must be set
        (self.dmacon & dmacon::DMAEN) != 0 && (self.dmacon & mask) != 0
    }

    /// Returns true if Blitter Nasty (BLTPRI, bit 10) is enabled
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & dmacon::BLTPRI) != 0
    }
}
