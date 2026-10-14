//! Agnus (MOS 8370 / 8371) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub use blitter;
pub use copper;
pub use dma;

use config::chip_ram::{OCS_DMA_ADDRESS_MASK, OCS_DMA_HIGH_MASK, OCS_DMA_LOW_MASK};
use config::custom_reg::agnus as reg;
use config::mask::dmacon;
pub use config::AgnusModel;
use config::{stage_mutation, tick_mutations, BeamPosition, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Agnus custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agnus {
    /// Active Agnus chip hardware model (8370 or 8371)
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
    /// Display Window Start ($08E, vertical DMA start)
    pub diwstrt: u16,
    /// Display Window Stop ($090, vertical DMA stop)
    pub diwstop: u16,
    /// Display Data Fetch Start ($092)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094)
    pub ddfstop: u16,
    /// Bitplane Control 0 ($100, latched by Agnus for DMA slot count)
    pub bplcon0: u16,
    /// Bitplane Modulo 1 ($108, odd bitplanes)
    pub bpl1mod: i16,
    /// Bitplane Modulo 2 ($10A, even bitplanes)
    pub bpl2mod: i16,
    /// Bitplane DMA pointers 1..6 ($0E0-$0F6)
    pub bplpt: [u32; 6],
    /// Sprite DMA pointers 0..7 ($120-$13E)
    pub sprpt: [u32; 8],
    /// Sprite position registers 0..7 ($140-$178)
    pub sprpos: [u16; 8],
    /// Sprite control registers 0..7 ($142-$17A)
    pub sprctl: [u16; 8],
    /// Audio DMA pointers 0..3 ($0A0, $0B0, $0C0, $0D0)
    pub audpt: [u32; 4],
    /// Audio DMA start/loop locations 0..3 (AUDxLCH/AUDxLCL)
    pub audlc: [u32; 4],
    /// Floppy Disk DMA pointer ($020-$022)
    pub dskpt: u32,
    /// Disk DMA request line asserted by Paula/Floppy (_DSKREQ pin)
    pub dsk_dma_req: bool,
    /// Pending Disk DMA Chip RAM address slot awarded on this cycle
    pub pending_dsk_dma_slot: Option<u32>,
    /// Vertical blanking interrupt request strobe
    pub vblank_irq: bool,
    /// Pending custom register write emitted by the Copper on this cycle
    pub pending_copper_write: Option<(u16, u16)>,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; Agnus::MUTATION_CAPACITY],
}

impl Agnus {
    /// Maximum horizontal Color Clock cycles per scanline (PAL)
    pub const PAL_LINE_CCKS: u16 = 227;
    /// Total vertical scanlines per frame (PAL)
    const PAL_FRAME_LINES: u16 = 312;
    /// Short scanline length in Color Clocks (NTSC standard)
    pub const NTSC_SHORT_LINE_CCKS: u16 = 227;
    /// Long scanline length in Color Clocks (NTSC interlace LOL bit active)
    pub const NTSC_LONG_LINE_CCKS: u16 = 228;
    /// Total vertical scanlines per frame (NTSC)
    const NTSC_FRAME_LINES: u16 = 262;

    /// Number of Color Clocks that Agnus's internal scheduling counter leads the visible display beam
    pub const VHPOSR_PIPELINE_LEAD_CCKS: u16 = 5;
    /// Number of Color Clocks after line wrap during which the vertical ripple counter is settling
    const VHPOSR_VERTICAL_SETTLE_CCKS: u16 = 1;

    /// Fixed-capacity in-flight register mutation buffer for Agnus (covers all writable registers)
    pub const MUTATION_CAPACITY: usize = 64;

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
            dsk_dma_req: false,
            pending_dsk_dma_slot: None,
            dmacon: 0,
            diwstrt: 0,
            diwstop: 0,
            ddfstrt: 0x0038,
            ddfstop: 0x00D0,
            bplcon0: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            bplpt: [0; 6],
            sprpt: [0; 8],
            sprpos: [0; 8],
            sprctl: [0; 8],
            audpt: [0; 4],
            audlc: [0; 4],
            dskpt: 0,
            mutations: [None; Self::MUTATION_CAPACITY],
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
        self.dsk_dma_req = false;
        self.pending_dsk_dma_slot = None;
        self.dmacon = 0;
        self.diwstrt = 0;
        self.diwstop = 0;
        self.ddfstrt = 0x0038;
        self.ddfstop = 0x00D0;
        self.bplcon0 = 0;
        self.bpl1mod = 0;
        self.bpl2mod = 0;
        self.bplpt.fill(0);
        self.sprpt.fill(0);
        self.sprpos.fill(0);
        self.sprctl.fill(0);
        self.audpt.fill(0);
        self.audlc.fill(0);
        self.dskpt = 0;
        self.vblank_irq = false;
        self.pending_copper_write = None;
        self.mutations = [None; Self::MUTATION_CAPACITY];
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
            AgnusModel::OcsNtsc8370 => Self::NTSC_FRAME_LINES,
            _ => Self::PAL_FRAME_LINES,
        };
        let line_ccks = match self.model {
            AgnusModel::OcsNtsc8370 => {
                if self.lol {
                    Self::NTSC_LONG_LINE_CCKS
                } else {
                    Self::NTSC_SHORT_LINE_CCKS
                }
            }
            _ => Self::PAL_LINE_CCKS,
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
        self.dma.step_cck();

        // Evaluate 8-tier Master DMA Bus Arbitration
        let copper_wants_bus = self.copper.is_active_fetching();
        let blitter_wants_bus = self.blitter.is_busy;
        let owner = self.dma.arbitrate(
            beam.hpos,
            beam.vpos,
            self.dsk_dma_req,
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
            dma::DmaChannel::Disk => {
                self.pending_dsk_dma_slot = Some(self.dskpt);
                self.dskpt = self.dskpt.wrapping_add(2) & OCS_DMA_ADDRESS_MASK;
            }
            dma::DmaChannel::Bitplane(plane) => {
                let p = plane as usize;
                if p < 6 && !chip_ram.is_empty() {
                    let word = config::chip_ram::read_u16(chip_ram, self.bplpt[p]);
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
                        & OCS_DMA_ADDRESS_MASK;
                }
            }
            dma::DmaChannel::Sprite(sprite) => {
                let s = sprite as usize;
                if s < 8 && !chip_ram.is_empty() {
                    let addr = (self.sprpt[s] & OCS_DMA_ADDRESS_MASK) as usize
                        & chip_ram.len().wrapping_sub(1);
                    if addr + 1 < chip_ram.len() {
                        self.sprpt[s] = self.sprpt[s].wrapping_add(2) & OCS_DMA_ADDRESS_MASK;
                    }
                }
            }
            dma::DmaChannel::Audio(ch) => {
                let c = ch as usize;
                if c < 4 && !chip_ram.is_empty() {
                    let addr = (self.audpt[c] & OCS_DMA_ADDRESS_MASK) as usize
                        & chip_ram.len().wrapping_sub(1);
                    if addr + 1 < chip_ram.len() {
                        self.audpt[c] = self.audpt[c].wrapping_add(2) & OCS_DMA_ADDRESS_MASK;
                    }
                }
            }
            _ => {}
        }

        self.pending_copper_write = self.copper.step_cck(beam, self.blitter.is_busy, chip_ram);

        // 3. Process and commit due register mutations
        let mut committed = [None; Self::MUTATION_CAPACITY];
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

    /// Sets the disk DMA request line asserted by Paula/Floppy
    #[inline]
    pub fn set_dsk_dma_req(&mut self, req: bool) {
        self.dsk_dma_req = req;
    }

    /// Polls and clears any Disk DMA Chip RAM address slot awarded on this cycle
    #[inline]
    pub fn poll_dsk_dma_slot(&mut self) -> Option<u32> {
        self.pending_dsk_dma_slot.take()
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
            AgnusModel::OcsNtsc8370 => Self::NTSC_FRAME_LINES,
            _ => Self::PAL_FRAME_LINES,
        };
        let line_ccks = match self.model {
            AgnusModel::OcsNtsc8370 => {
                if self.lol {
                    Self::NTSC_LONG_LINE_CCKS
                } else {
                    Self::NTSC_SHORT_LINE_CCKS
                }
            }
            _ => Self::PAL_LINE_CCKS,
        };

        let mut h = self.hpos + Self::VHPOSR_PIPELINE_LEAD_CCKS;
        let mut v = self.vpos;
        if h >= line_ccks {
            h -= line_ccks;
            v = v.wrapping_add(1);
            if v >= max_lines {
                v = 0;
            }
        }

        let effective_v = if h <= Self::VHPOSR_VERTICAL_SETTLE_CCKS {
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
            reg::DMACONR => self.read_dmaconr(),
            reg::VPOSR => self.vposr(),
            reg::VHPOSR => self.vhposr(),
            _ => config::OPEN_BUS_WORD,
        }
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode.
    /// Returns `Some((offset, val))` if the register was committed immediately (e.g. overflow fallback or delay 0),
    /// or `None` if staged in the mutation pipeline.
    pub fn write_register(&mut self, offset: u16, val: u16) -> Option<(u16, u16)> {
        let offset = offset & 0x1FE;
        let (delay, mode) = match offset {
            reg::DMACON => (2, MutationMode::OverwritePending),
            reg::COPJMP1 | reg::COPJMP2 => (1, MutationMode::OverwritePending),
            reg::BLTSIZE => (1, MutationMode::OverwritePending),
            reg::BPLCON0 => (4, MutationMode::OverwritePending), // BPLCON0 (Agnus DMA allocation)
            reg::DIWSTRT | reg::DIWSTOP => (4, MutationMode::OverwritePending),
            reg::DDFSTRT | reg::DDFSTOP => (4, MutationMode::OverwritePending),
            reg::BPL1MOD | reg::BPL2MOD => (2, MutationMode::OverwritePending),
            reg::BLTCON0
            | reg::BLTCON1
            | reg::BLTAFWM
            | reg::BLTALWM
            | reg::BLTCPTH
            | reg::BLTCPTL
            | reg::BLTBPTH
            | reg::BLTBPTL
            | reg::BLTAPTH
            | reg::BLTAPTL
            | reg::BLTDPTH
            | reg::BLTDPTL => (2, MutationMode::OverwritePending),
            reg::BLTCMOD | reg::BLTBMOD | reg::BLTAMOD | reg::BLTDMOD => {
                (2, MutationMode::OverwritePending)
            }
            reg::BLTCDAT | reg::BLTBDAT | reg::BLTADAT => (1, MutationMode::Pipeline),
            reg::COP1LCH | reg::COP1LCL | reg::COP2LCH | reg::COP2LCL => {
                (2, MutationMode::OverwritePending)
            }
            reg::BPL1PTH
            | reg::BPL1PTL
            | reg::BPL2PTH
            | reg::BPL2PTL
            | reg::BPL3PTH
            | reg::BPL3PTL
            | reg::BPL4PTH
            | reg::BPL4PTL
            | reg::BPL5PTH
            | reg::BPL5PTL
            | reg::BPL6PTH
            | reg::BPL6PTL => (2, MutationMode::OverwritePending),
            reg::SPR0PTH
            | reg::SPR0PTL
            | reg::SPR1PTH
            | reg::SPR1PTL
            | reg::SPR2PTH
            | reg::SPR2PTL
            | reg::SPR3PTH
            | reg::SPR3PTL
            | reg::SPR4PTH
            | reg::SPR4PTL
            | reg::SPR5PTH
            | reg::SPR5PTL
            | reg::SPR6PTH
            | reg::SPR6PTL
            | reg::SPR7PTH
            | reg::SPR7PTL => (2, MutationMode::OverwritePending),
            reg::DSKPTH | reg::DSKPTL => (2, MutationMode::OverwritePending),
            reg::AUD0LCH
            | reg::AUD0LCL
            | reg::AUD1LCH
            | reg::AUD1LCL
            | reg::AUD2LCH
            | reg::AUD2LCL
            | reg::AUD3LCH
            | reg::AUD3LCL => (2, MutationMode::OverwritePending),
            _ => (2, MutationMode::OverwritePending),
        };

        if !stage_mutation(&mut self.mutations, offset, val, delay, mode) {
            self.commit_register_write(offset, val);
            Some((offset, val))
        } else {
            None
        }
    }

    fn write_dma_pointer_high(pointer: u32, value: u16) -> u32 {
        (pointer & u32::from(OCS_DMA_LOW_MASK)) | (u32::from(value & OCS_DMA_HIGH_MASK) << 16)
    }
    fn write_dma_pointer_low(pointer: u32, value: u16) -> u32 {
        (pointer & (OCS_DMA_ADDRESS_MASK & 0xFFFF_0000)) | u32::from(value & OCS_DMA_LOW_MASK)
    }
    /// Commits a register write directly into active Agnus silicon state
    pub fn commit_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            reg::DMACON => {
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
            reg::COPCON => self.copper.set_copcon(val),
            reg::COP1LCH => {
                self.copper.cop1lc = Self::write_dma_pointer_high(self.copper.cop1lc, val)
            }
            reg::COP1LCL => {
                self.copper.cop1lc = Self::write_dma_pointer_low(self.copper.cop1lc, val)
            }
            reg::COP2LCH => {
                self.copper.cop2lc = Self::write_dma_pointer_high(self.copper.cop2lc, val)
            }
            reg::COP2LCL => {
                self.copper.cop2lc = Self::write_dma_pointer_low(self.copper.cop2lc, val)
            }
            reg::COPJMP1 => self.strobe_copjmp1(),
            reg::COPJMP2 => self.strobe_copjmp2(),
            reg::DIWSTRT => {
                self.diwstrt = val;
                self.dma.set_vstart(val >> 8);
            }
            reg::DIWSTOP => {
                self.diwstop = val;
                let vstop = (val >> 8) | (if val & 0x8000 == 0 { 0x100 } else { 0 });
                self.dma.set_vstop(vstop);
            }
            reg::DDFSTRT => {
                self.ddfstrt = val & 0x00FC;
                self.dma.set_ddfstrt(val & 0x00FC);
            }
            reg::DDFSTOP => {
                self.ddfstop = val & 0x00FC;
                self.dma.set_ddfstop(val & 0x00FC);
            }
            reg::BPLCON0 => self.set_bplcon0(val),
            reg::BPL1MOD => self.bpl1mod = val as i16,
            reg::BPL2MOD => self.bpl2mod = val as i16,

            // Blitter registers
            reg::BLTCON0 => self.blitter.bltcon0 = val,
            reg::BLTCON1 => self.blitter.bltcon1 = val,
            reg::BLTAFWM => self.blitter.bltafwm = val,
            reg::BLTALWM => self.blitter.bltalwm = val,
            reg::BLTCPTH => {
                self.blitter.bltcpt = Self::write_dma_pointer_high(self.blitter.bltcpt, val)
            }
            reg::BLTCPTL => {
                self.blitter.bltcpt = Self::write_dma_pointer_low(self.blitter.bltcpt, val)
            }
            reg::BLTBPTH => {
                self.blitter.bltbpt = Self::write_dma_pointer_high(self.blitter.bltbpt, val)
            }
            reg::BLTBPTL => {
                self.blitter.bltbpt = Self::write_dma_pointer_low(self.blitter.bltbpt, val)
            }
            reg::BLTAPTH => {
                self.blitter.bltapt = Self::write_dma_pointer_high(self.blitter.bltapt, val)
            }
            reg::BLTAPTL => {
                self.blitter.bltapt = Self::write_dma_pointer_low(self.blitter.bltapt, val)
            }
            reg::BLTDPTH => {
                self.blitter.bltdpt = Self::write_dma_pointer_high(self.blitter.bltdpt, val)
            }
            reg::BLTDPTL => {
                self.blitter.bltdpt = Self::write_dma_pointer_low(self.blitter.bltdpt, val)
            }
            reg::BLTSIZE => self.blitter.trigger_blit(val),
            reg::BLTCMOD => self.blitter.bltcmod = val as i16,
            reg::BLTBMOD => self.blitter.bltbmod = val as i16,
            reg::BLTAMOD => self.blitter.bltamod = val as i16,
            reg::BLTDMOD => self.blitter.bltdmod = val as i16,
            reg::BLTCDAT => self.blitter.bltcdat = val,
            reg::BLTBDAT => self.blitter.bltbdat = val,
            reg::BLTADAT => self.blitter.bltadat = val,

            // Bitplane pointers
            reg::BPL1PTH
            | reg::BPL1PTL
            | reg::BPL2PTH
            | reg::BPL2PTL
            | reg::BPL3PTH
            | reg::BPL3PTL
            | reg::BPL4PTH
            | reg::BPL4PTL
            | reg::BPL5PTH
            | reg::BPL5PTL
            | reg::BPL6PTH
            | reg::BPL6PTL => {
                let plane = ((offset - reg::BPL1PTH) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if plane < 6 {
                    if is_low {
                        self.bplpt[plane] = Self::write_dma_pointer_low(self.bplpt[plane], val);
                    } else {
                        self.bplpt[plane] = Self::write_dma_pointer_high(self.bplpt[plane], val);
                    }
                }
            }

            // Sprite pointers
            reg::SPR0PTH
            | reg::SPR0PTL
            | reg::SPR1PTH
            | reg::SPR1PTL
            | reg::SPR2PTH
            | reg::SPR2PTL
            | reg::SPR3PTH
            | reg::SPR3PTL
            | reg::SPR4PTH
            | reg::SPR4PTL
            | reg::SPR5PTH
            | reg::SPR5PTL
            | reg::SPR6PTH
            | reg::SPR6PTL
            | reg::SPR7PTH
            | reg::SPR7PTL => {
                let sprite = ((offset - reg::SPR0PTH) / 4) as usize;
                let is_low = (offset & 2) != 0;
                if sprite < 8 {
                    if is_low {
                        self.sprpt[sprite] = Self::write_dma_pointer_low(self.sprpt[sprite], val);
                    } else {
                        self.sprpt[sprite] = Self::write_dma_pointer_high(self.sprpt[sprite], val);
                    }
                }
            }

            // Sprite position and control latches
            reg::SPR0POS
            | reg::SPR0CTL
            | reg::SPR1POS
            | reg::SPR1CTL
            | reg::SPR2POS
            | reg::SPR2CTL
            | reg::SPR3POS
            | reg::SPR3CTL
            | reg::SPR4POS
            | reg::SPR4CTL
            | reg::SPR5POS
            | reg::SPR5CTL
            | reg::SPR6POS
            | reg::SPR6CTL
            | reg::SPR7POS
            | reg::SPR7CTL => {
                let sprite = ((offset - reg::SPR0POS) / 8) as usize;
                if (offset & 2) == 0 {
                    self.sprpos[sprite] = val;
                } else {
                    self.sprctl[sprite] = val;
                }
            }

            // Floppy Disk DMA pointer
            reg::DSKPTH => {
                self.dskpt = Self::write_dma_pointer_high(self.dskpt, val);
            }
            reg::DSKPTL => {
                self.dskpt = Self::write_dma_pointer_low(self.dskpt, val);
            }

            // Audio DMA location pointers (AUD0LCH/LCL..AUD3LCH/LCL)
            reg::AUD0LCH
            | reg::AUD0LCL
            | reg::AUD1LCH
            | reg::AUD1LCL
            | reg::AUD2LCH
            | reg::AUD2LCL
            | reg::AUD3LCH
            | reg::AUD3LCL => {
                let ch = ((offset - reg::AUD0LCH) / 0x10) as usize;
                if ch < 4 {
                    if (offset & 2) == 0 {
                        self.audlc[ch] = Self::write_dma_pointer_high(self.audlc[ch], val);
                    } else {
                        self.audlc[ch] = Self::write_dma_pointer_low(self.audlc[ch], val);
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
        let master_enabled = (self.dmacon & dmacon::DMAEN) != 0;
        let channel_enabled = (self.dmacon & mask) != 0;
        master_enabled && channel_enabled
    }

    /// Returns true if Blitter Nasty (BLTPRI, bit 10) is enabled
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & dmacon::BLTPRI) != 0
    }

    /// Triggers COPJMP1 strobe: restarts Copper execution at COP1LC
    #[inline]
    pub fn strobe_copjmp1(&mut self) {
        self.copper.restart_list1();
    }

    /// Triggers COPJMP2 strobe: restarts Copper execution at COP2LC
    #[inline]
    pub fn strobe_copjmp2(&mut self) {
        self.copper.restart_list2();
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline(always)]
    pub fn dmaconr(&self) -> u16 {
        self.read_dmaconr()
    }

    /// Reads DMA Control and Blitter status without side-effects for debugging
    #[inline(always)]
    pub fn dmaconr_debug(&self) -> u16 {
        self.read_dmaconr()
    }

    /// Reads Vertical and Horizontal beam position register without side-effects for debugging
    #[inline(always)]
    pub fn vhposr_debug(&self) -> u16 {
        self.vhposr()
    }

    /// Reads Vertical position register and chip ID without side-effects for debugging
    #[inline(always)]
    pub fn vposr_debug(&self) -> u16 {
        self.vposr()
    }
}
