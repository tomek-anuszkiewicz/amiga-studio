//! Agnus (MOS 8370 / 8371) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub use blitter;
pub use copper;
pub use dma;

use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use config::mask::dmacon;
pub use config::AgnusModel;
use config::{
    tick_mutations, DelayedMutation, AUDIO_CHANNEL_COUNT, OCS_BITPLANE_COUNT, SPRITE_COUNT,
};
use dma::{DDFSTOP_DEFAULT, DDFSTRT_DEFAULT};
use serde::{Deserialize, Serialize};

mod pins;
pub use pins::AgnusPins;

mod blitter_registers;
mod copper_registers;
mod register_apply;
mod registers;

/// Agnus custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agnus {
    /// Motherboard-facing signal state.
    pub pins: AgnusPins,
    /// Granted audio/sprite RGA and data output, held until the router accepts it.
    /// Scheduling and register selection belong to the owning DMA engine.
    pub pending_dma_write: Option<(u16, u16)>,
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
    pub bplpt: [u32; OCS_BITPLANE_COUNT],
    /// Sprite DMA pointers 0..7 ($120-$13E)
    pub sprpt: [u32; SPRITE_COUNT],
    /// Sprite position registers 0..7 ($140-$178)
    pub sprpos: [u16; SPRITE_COUNT],
    /// Sprite control registers 0..7 ($142-$17A)
    pub sprctl: [u16; SPRITE_COUNT],
    /// Audio DMA pointers 0..3 ($0A0, $0B0, $0C0, $0D0)
    pub audpt: [u32; AUDIO_CHANNEL_COUNT],
    /// Audio DMA start/loop locations 0..3 (AUDxLCH/AUDxLCL)
    pub audlc: [u32; AUDIO_CHANNEL_COUNT],
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
            pins: AgnusPins::default(),
            pending_dma_write: None,
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
            ddfstrt: DDFSTRT_DEFAULT,
            ddfstop: DDFSTOP_DEFAULT,
            bplcon0: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            bplpt: [0; OCS_BITPLANE_COUNT],
            sprpt: [0; SPRITE_COUNT],
            sprpos: [0; SPRITE_COUNT],
            sprctl: [0; SPRITE_COUNT],
            audpt: [0; AUDIO_CHANNEL_COUNT],
            audlc: [0; AUDIO_CHANNEL_COUNT],
            dskpt: 0,
            mutations: [None; Self::MUTATION_CAPACITY],
        }
    }

    /// Resets Agnus registers and beam counters to power-on defaults
    pub fn reset(&mut self) {
        self.pins = AgnusPins::default();
        self.pending_dma_write = None;
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
        self.ddfstrt = DDFSTRT_DEFAULT;
        self.ddfstop = DDFSTOP_DEFAULT;
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
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; Self::MUTATION_CAPACITY] {
        self.step_cck_ram(&mut [])
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers with Chip RAM access,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck_ram(
        &mut self,
        chip_ram: &mut [u8],
    ) -> [Option<(u16, u16)>; Self::MUTATION_CAPACITY] {
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
        self.dma.step_cck();

        // Evaluate 8-tier Master DMA Bus Arbitration
        let copper_wants_bus = self.copper.is_active_fetching();
        let blitter_wants_bus = self.blitter.is_busy;
        let owner = self.dma.arbitrate(
            self.hpos,
            self.vpos,
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
                self.dskpt = self.dskpt.wrapping_add(2) & MASK_OCS_DMA_ADDRESS;
            }
            dma::DmaChannel::Bitplane(plane) => {
                let p = plane as usize;
                if p < OCS_BITPLANE_COUNT && !chip_ram.is_empty() && self.pending_bpl_dma.is_none()
                {
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
                        & MASK_OCS_DMA_ADDRESS;
                }
            }
            dma::DmaChannel::Sprite(sprite) => {
                let s = sprite as usize;
                if s < SPRITE_COUNT && !chip_ram.is_empty() {
                    let addr = (self.sprpt[s] & MASK_OCS_DMA_ADDRESS) as usize
                        & chip_ram.len().wrapping_sub(1);
                    if addr + 1 < chip_ram.len() {
                        self.sprpt[s] = self.sprpt[s].wrapping_add(2) & MASK_OCS_DMA_ADDRESS;
                    }
                }
            }
            dma::DmaChannel::Audio(ch) => {
                let c = ch as usize;
                if c < AUDIO_CHANNEL_COUNT && !chip_ram.is_empty() {
                    let addr = (self.audpt[c] & MASK_OCS_DMA_ADDRESS) as usize
                        & chip_ram.len().wrapping_sub(1);
                    if addr + 1 < chip_ram.len() {
                        self.audpt[c] = self.audpt[c].wrapping_add(2) & MASK_OCS_DMA_ADDRESS;
                    }
                }
            }
            _ => {}
        }

        if self.pending_copper_write.is_none() {
            self.pending_copper_write =
                self.copper
                    .step_cck(self.hpos, self.vpos, self.blitter.is_busy, chip_ram);
        }

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
            self.apply_register_write(item.0, item.1);
        }

        self.drive_sync_strobe();
        self.drive_timing_pins(line_ccks, max_lines);
        committed
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

    /// Returns true if Chip RAM is currently locked by custom chip DMA
    #[inline]
    pub fn is_chip_ram_blocked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Reloads audio DMA pointer from latched start address (AUDxLC) for channel `channel` (0..3)
    #[inline]
    pub fn reload_audio_ptr(&mut self, channel: usize) {
        if channel < AUDIO_CHANNEL_COUNT {
            self.audpt[channel] = self.audlc[channel];
        }
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
        let master_enabled = (self.dmacon & dmacon::BIT_DMAEN) != 0;
        let channel_enabled = (self.dmacon & mask) != 0;
        master_enabled && channel_enabled
    }

    /// Returns true if Blitter Nasty (BLTPRI, bit 10) is enabled
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & dmacon::BIT_BLTPRI) != 0
    }
}
