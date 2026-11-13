//! Agnus (MOS 8370 / 8371) Architecture & Subsystem Coordinator
//!
//! Master bus controller, DMA arbiter, raster beam counter, and host for
//! the Copper coprocessor and 4-channel DMA Blitter.

pub mod blitter;
pub mod copper;
pub mod dma;
pub use blitter::Blitter;
pub use copper::Copper;
pub use dma::DmaScheduler;

use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use config::mask::dmacon;
pub use config::AgnusModel;
use config::{MutationQueue, AUDIO_CHANNEL_COUNT, OCS_BITPLANE_COUNT, SPRITE_COUNT};
use serde::{Deserialize, Serialize};

mod pins;
pub use pins::AgnusPins;

mod beam;
mod blitter_registers;
mod copper_registers;
mod dma_audio;
mod dma_disk;
mod dma_sprite;
mod register_apply;
mod register_bits;
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
    /// Long Line state for NTSC (alternates 227 and 228 CCK scanlines)
    pub lol: bool,
    /// True if Chip RAM is currently blocked by custom chip DMA
    pub chip_ram_blocked: bool,
    /// Pending bitplane DMA word fetched from Chip RAM to be routed to Denise BPLxDAT
    pub pending_bpl_dma: Option<(u8, u16)>,

    // --- Active Latched Registers (Read is NOW) ---
    /// Display Window Start ($08E, vertical DMA start)
    pub diwstrt: u16,
    /// Display Window Stop ($090, vertical DMA stop)
    pub diwstop: u16,
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
    /// Vertical start/stop latch for each sprite DMA channel.
    pub sprite_dma_active: [bool; SPRITE_COUNT],
    /// Audio DMA pointers 0..3 ($0A0, $0B0, $0C0, $0D0)
    pub audpt: [u32; AUDIO_CHANNEL_COUNT],
    /// Audio DMA start/loop locations 0..3 (AUDxLCH/AUDxLCL)
    pub audlc: [u32; AUDIO_CHANNEL_COUNT],
    pub audio_dma_request: [bool; AUDIO_CHANNEL_COUNT],
    pub audio_dma_restart: [bool; AUDIO_CHANNEL_COUNT],
    /// Floppy Disk DMA pointer ($020-$022)
    pub dskpt: u32,
    /// Three line-relative requests and directions decoded from captured DMAL.
    pub disk_dma_requests: [bool; config::dmal::DISK_SLOT_COUNT],
    pub disk_dma_write: [bool; config::dmal::DISK_SLOT_COUNT],
    /// Pending Disk DMA Chip RAM address slot awarded on this cycle
    pub pending_dsk_dma_slot: Option<u32>,
    /// Pending custom register write emitted by the Copper on this cycle
    pub pending_copper_write: Option<(u16, u16)>,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    pub mutations: MutationQueue<{ Agnus::MUTATION_CAPACITY }>,
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
            disk_dma_requests: [false; config::dmal::DISK_SLOT_COUNT],
            disk_dma_write: [false; config::dmal::DISK_SLOT_COUNT],
            pending_dsk_dma_slot: None,
            diwstrt: 0,
            diwstop: 0,
            bpl1mod: 0,
            bpl2mod: 0,
            bplpt: [0; OCS_BITPLANE_COUNT],
            sprpt: [0; SPRITE_COUNT],
            sprpos: [0; SPRITE_COUNT],
            sprctl: [0; SPRITE_COUNT],
            sprite_dma_active: [false; SPRITE_COUNT],
            audpt: [0; AUDIO_CHANNEL_COUNT],
            audlc: [0; AUDIO_CHANNEL_COUNT],
            audio_dma_request: [false; AUDIO_CHANNEL_COUNT],
            audio_dma_restart: [false; AUDIO_CHANNEL_COUNT],
            dskpt: 0,
            mutations: MutationQueue::default(),
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
        self.disk_dma_requests = [false; config::dmal::DISK_SLOT_COUNT];
        self.disk_dma_write = [false; config::dmal::DISK_SLOT_COUNT];
        self.pending_dsk_dma_slot = None;
        self.diwstrt = 0;
        self.diwstop = 0;
        self.bpl1mod = 0;
        self.bpl2mod = 0;
        self.bplpt.fill(0);
        self.sprpt.fill(0);
        self.sprpos.fill(0);
        self.sprctl.fill(0);
        self.sprite_dma_active.fill(false);
        self.audpt.fill(0);
        self.audlc.fill(0);
        self.audio_dma_request.fill(false);
        self.audio_dma_restart.fill(false);
        self.dskpt = 0;
        self.pending_copper_write = None;
        self.mutations.clear();
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Borrows all writes committed this cycle until the next mutable chip access.
    #[inline]
    pub fn step_cck(&mut self) -> &[(u16, u16)] {
        self.step_cck_ram(&mut [])
    }

    /// Advances raster beam position, steps embedded coprocessors and schedulers with Chip RAM access,
    /// and processes in-flight register mutations by 1 Color Clock.
    /// Borrows all writes committed this cycle until the next mutable chip access.
    pub fn step_cck_ram(&mut self, chip_ram: &mut [u8]) -> &[(u16, u16)] {
        self.advance_beam();
        self.sample_audio_request();
        self.sample_disk_request();
        let line_ccks = self.line_ccks();
        let max_lines = self.field_lines();

        // 2. Step embedded coprocessors and schedulers
        self.copper
            .step_cck(self.hpos, self.vpos, self.blitter.is_busy);

        // Evaluate 8-tier Master DMA Bus Arbitration
        let copper_wants_bus =
            self.pending_copper_write.is_none() && self.copper.is_active_fetching();
        let blitter_wants_bus = self.blitter.wants_bus();
        let sprite_requests = self.sprite_requests();
        let owner = self.dma.step_cck(
            self.hpos,
            self.vpos,
            self.disk_request_for_slot(),
            self.audio_requests(),
            sprite_requests,
            copper_wants_bus,
            blitter_wants_bus,
            self.dma.cpu_request_pending,
            line_ccks,
        );
        self.chip_ram_blocked = self.dma.chip_ram_blocked;

        match owner {
            dma::DmaChannel::Copper => {
                if !chip_ram.is_empty() {
                    let word = config::chip_ram::read_u16(chip_ram, self.copper.cop_pc);
                    self.pending_copper_write = self.copper.receive_dma_word(
                        word,
                        self.hpos,
                        self.vpos,
                        self.blitter.is_busy,
                    );
                }
            }
            dma::DmaChannel::Disk => self.grant_disk_word(chip_ram),
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
            dma::DmaChannel::Sprite(sprite) => self.fetch_sprite_word(sprite as usize, chip_ram),
            dma::DmaChannel::Audio(ch) => self.fetch_audio_word(ch as usize, chip_ram),
            _ => {}
        }

        self.blitter
            .step_cck_ram(chip_ram, owner == dma::DmaChannel::Blitter);

        // 3. Process and commit due register mutations
        self.mutations.tick();
        for index in 0..self.mutations.due().len() {
            let (reg, value) = self.mutations.due()[index];
            self.apply_register_write(reg, value);
        }

        self.drive_sync_strobe();
        self.drive_timing_pins(line_ccks, max_lines);
        self.mutations.due()
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

    /// Queries whether a specific DMA channel is enabled in DMACON
    #[inline]
    pub fn is_dma_enabled(&self, mask: u16) -> bool {
        let master_enabled = (self.dma.dmacon & dmacon::BIT_DMAEN) != 0;
        let channel_enabled = (self.dma.dmacon & mask) != 0;
        master_enabled && channel_enabled
    }

    /// Returns true if Blitter Nasty (BLTPRI, bit 10) is enabled
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dma.dmacon & dmacon::BIT_BLTPRI) != 0
    }
}
