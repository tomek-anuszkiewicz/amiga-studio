//! Agnus DMA Scheduler & Bus Arbitration Engine
//!
//! Evaluates the horizontal scanline DMA slot schedule (227.5 CCKs per line),
//! channel gating via DMACON, dynamic Bitplane DMA cycle stealing in the DDF window,
//! 8-tier priority arbitration, Blitter Nasty vs Normal mode with 3-cycle CPU starvation yield,
//! and Chip RAM bus contention against the CPU.

use config::mask::{bplcon0, dmacon};
use config::{display_window, AUDIO_CHANNEL_COUNT, SPRITE_COUNT};
use serde::{Deserialize, Serialize};

use crate::register_bits::display_fetch::{HIRES_FETCH_PERIOD_CCK, LORES_FETCH_PERIOD_CCK};

// HRM 6, Figures 6-11/6-12: one-based planes; zero is an unused slot.
const LORES_FETCH_PLANES: [u8; 8] = [0, 4, 6, 2, 0, 3, 5, 1];
const HIRES_FETCH_PLANES: [u8; 4] = [4, 2, 3, 1];
const COPPER_MEMORY_PHASE: u16 = 0;
const COPPER_END_OF_LINE_EXCLUSION: u16 = 0xE0;

pub const REFRESH_FIRST: u16 = 1;
pub const REFRESH_SECOND: u16 = 3;
const REFRESH_THIRD: u16 = 5;
const REFRESH_LATE: u16 = 0xE2;
const DISK_FIRST: u16 = 7;
const DISK_SECOND: u16 = 9;
const DISK_THIRD: u16 = 11;
const AUDIO0_SLOT: u16 = 13;
const AUDIO1_SLOT: u16 = 15;
const AUDIO2_SLOT: u16 = 17;
const AUDIO3_SLOT: u16 = 19;
const DMA_SLOT_PERIOD_CCK: u16 = 2;
const DMA_ACTIVE_PHASE: u16 = 1;
pub const SPRITE_SLOT_PERIOD_CCK: u16 = 4;
/// CCK ranges and slots for fixed DMA channels on each horizontal scanline (Figure 6-9 in Commodore HRM)
pub const HPOS_REFRESH_SLOTS: [u16; 4] =
    [REFRESH_FIRST, REFRESH_SECOND, REFRESH_THIRD, REFRESH_LATE];
pub const HPOS_DISK_SLOTS: [u16; 3] = [DISK_FIRST, DISK_SECOND, DISK_THIRD];
pub const HPOS_AUDIO_SLOTS: [u16; AUDIO_CHANNEL_COUNT] =
    [AUDIO0_SLOT, AUDIO1_SLOT, AUDIO2_SLOT, AUDIO3_SLOT];
pub const HPOS_SPRITE_START: u16 = 21;
pub const HPOS_SPRITE_END: u16 = 51;

/// Normal Blitter mode (BLTPRI = 0) yields to CPU after 3 consecutive memory cycles of CPU starvation
pub const BLITTER_STARVATION_YIELD_CYCLES: u8 = 3;

/// Default display data fetch boundaries for OCS standard Low-Resolution screen
pub const DDFSTRT_DEFAULT: u16 = 0x0038;
pub const DDFSTOP_DEFAULT: u16 = 0x00D0;

/// Custom chip DMA channels in strict priority order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmaChannel {
    /// Four DRAM refresh opportunities per line, independent of DMA enables.
    Refresh,
    /// Three disk word opportunities at HPOS 7, 9 and 11.
    Disk,
    /// One word opportunity per audio channel at HPOS 13, 15, 17 and 19.
    Audio(u8),
    /// Bitplane DMA channels 0 to 5 (Planes 1 to 6, dynamic slots in DDF window, priority 4)
    Bitplane(u8),
    /// Two word opportunities per sprite within HPOS 21 through 51.
    Sprite(u8),
    /// Copper coprocessor DMA (priority 6)
    Copper,
    /// 4-channel Blitter DMA (priority 7)
    Blitter,
    /// CPU access (bus awarded to CPU, priority 8)
    Cpu,
    /// Bus idle / unassigned
    None,
}

/// Agnus DMA scheduler and bus arbitration state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmaScheduler {
    /// Active DMA Control register state (DMACON / DMACONR)
    pub dmacon: u16,
    /// Bitplane Control 0 ($100, bits 12..14 planecount, bit 15 HIRES)
    pub bplcon0: u16,
    /// Vertical display window start (DIWSTRT high byte)
    pub vstart: u16,
    /// Vertical display window stop (DIWSTOP high byte and implicit OCS ninth bit)
    pub vstop: u16,
    /// Display Data Fetch Start ($092, default $0038)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094, default $00D0)
    pub ddfstop: u16,
    /// Consecutive memory cycles where CPU requested the bus but was held off by normal Blitter (BLTPRI=0)
    pub cpu_starvation_counter: u8,
    /// A refused production CPU access remains asserted until acceptance.
    pub cpu_request_pending: bool,
    /// Channel that currently owns the Chip RAM bus on this cycle
    pub current_owner: DmaChannel,
    /// True if Chip RAM is currently blocked from CPU access
    pub chip_ram_blocked: bool,
}

impl Default for DmaScheduler {
    fn default() -> Self {
        Self {
            dmacon: 0,
            bplcon0: 0,
            vstart: 0,
            vstop: display_window::BIT_VSTOP_EXTENSION,
            ddfstrt: DDFSTRT_DEFAULT,
            ddfstop: DDFSTOP_DEFAULT,
            cpu_starvation_counter: 0,
            cpu_request_pending: false,
            current_owner: DmaChannel::Cpu,
            chip_ram_blocked: false,
        }
    }
}

impl DmaScheduler {
    /// Creates a new uninitialized DMA scheduler
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets DMACON and internal counters to power-on defaults (all DMA disabled: $0000)
    pub fn reset(&mut self) {
        self.dmacon = 0;
        self.bplcon0 = 0;
        self.vstart = 0;
        self.vstop = display_window::BIT_VSTOP_EXTENSION;
        self.ddfstrt = DDFSTRT_DEFAULT;
        self.ddfstop = DDFSTOP_DEFAULT;
        self.cpu_starvation_counter = 0;
        self.cpu_request_pending = false;
        self.current_owner = DmaChannel::Cpu;
        self.chip_ram_blocked = false;
    }

    /// Sets BPLCON0 register value
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
    }

    /// Gary presents the CPU request after the current CCK bus selection.
    /// Repeated attempts within a CCK do not count as additional denials.
    pub fn receive_cpu_bus_wait(&mut self, waiting: bool) {
        if waiting && !self.cpu_request_pending {
            self.cpu_starvation_counter = 1;
        } else if !waiting {
            self.cpu_starvation_counter = 0;
        }
        self.cpu_request_pending = waiting;
    }

    /// Sets the vertical bitplane DMA start line.
    #[inline]
    pub fn set_vstart(&mut self, vstart: u16) {
        self.vstart = vstart;
    }

    /// Sets the vertical bitplane DMA stop line.
    #[inline]
    pub fn set_vstop(&mut self, vstop: u16) {
        self.vstop = vstop;
    }

    /// Sets DDFSTRT register value ($092)
    #[inline]
    pub fn set_ddfstrt(&mut self, val: u16) {
        self.ddfstrt = val;
    }

    /// Sets DDFSTOP register value ($094)
    #[inline]
    pub fn set_ddfstop(&mut self, val: u16) {
        self.ddfstop = val;
    }

    /// Returns the active number of bitplane DMA channels (0..6)
    #[inline]
    pub fn planecount(&self) -> u8 {
        ((self.bplcon0 >> bplcon0::BIT_BPU_SHIFT) & (bplcon0::MASK_BPU >> bplcon0::BIT_BPU_SHIFT))
            as u8
    }

    /// Returns true if High-Resolution mode is active (BPLCON0 bit 15)
    #[inline]
    pub fn is_hires(&self) -> bool {
        (self.bplcon0 & bplcon0::BIT_HIRES) != 0
    }

    /// DDFSTOP finishes an eight-CCK fetch group (two words per plane in hires).
    #[inline]
    pub fn is_in_ddf_window(&self, hpos: u16) -> bool {
        if hpos < self.ddfstrt || self.ddfstop < self.ddfstrt {
            return false;
        }
        let group_start = hpos - (hpos - self.ddfstrt) % LORES_FETCH_PERIOD_CCK;
        group_start <= self.ddfstop
    }

    /// Modulo belongs to each plane's final word, not the first hires half-group.
    #[inline]
    pub fn is_last_bpl_block(&self, hpos: u16) -> bool {
        if !self.is_in_ddf_window(hpos) {
            return false;
        }
        let phase = (hpos - self.ddfstrt) % LORES_FETCH_PERIOD_CCK;
        let group_start = hpos - phase;
        group_start.wrapping_add(LORES_FETCH_PERIOD_CCK) > self.ddfstop
            && (!self.is_hires() || phase >= HIRES_FETCH_PERIOD_CCK)
    }

    /// Returns true if the vertical position is inside the DIW bitplane DMA window.
    #[inline]
    pub fn is_in_vertical_display(&self, vpos: u16) -> bool {
        vpos >= self.vstart && (vpos < self.vstop || self.vstop < self.vstart)
    }

    /// Updates DMACON register following SET/CLR bit 15 logic
    pub fn write_dmacon(&mut self, val: u16) {
        if (val & dmacon::BIT_SET_CLR) != 0 {
            // SET bits
            self.dmacon |= val & dmacon::MASK_WRITE;
        } else {
            // CLR bits
            self.dmacon &= !(val & dmacon::MASK_WRITE);
        }
    }

    /// Returns true if the master DMA enable bit (DMAEN, bit 9) is asserted
    #[inline]
    pub fn is_dma_enabled(&self) -> bool {
        (self.dmacon & dmacon::BIT_DMAEN) != 0
    }

    /// Returns true if Blitter Nasty mode (BLTPRI, bit 10) is active
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & dmacon::BIT_BLTPRI) != 0
    }

    /// Checks whether a specific DMA channel is enabled in DMACON
    pub fn is_channel_enabled(&self, channel: DmaChannel) -> bool {
        if !self.is_dma_enabled() {
            return false;
        }
        match channel {
            DmaChannel::Refresh => true,
            DmaChannel::Disk => (self.dmacon & dmacon::BIT_DSKEN) != 0,
            DmaChannel::Audio(ch) => {
                (self.dmacon & (dmacon::BIT_AUD0EN << (ch.min((AUDIO_CHANNEL_COUNT - 1) as u8))))
                    != 0
            }
            DmaChannel::Sprite(_) => (self.dmacon & dmacon::BIT_SPREN) != 0,
            DmaChannel::Bitplane(_) => (self.dmacon & dmacon::BIT_BPLEN) != 0,
            DmaChannel::Copper => (self.dmacon & dmacon::BIT_COPEN) != 0,
            DmaChannel::Blitter => (self.dmacon & dmacon::BIT_BLTEN) != 0,
            DmaChannel::Cpu | DmaChannel::None => true,
        }
    }

    /// Returns the fixed DMA channel mapped to a horizontal Color Clock slot (HPOS)
    fn fixed_slot_for_hpos(hpos: u16, line_ccks: u16) -> Option<DmaChannel> {
        if hpos.wrapping_add(1) == line_ccks {
            return Some(DmaChannel::Refresh);
        }
        match hpos {
            REFRESH_FIRST | REFRESH_SECOND | REFRESH_THIRD => Some(DmaChannel::Refresh),
            DISK_FIRST | DISK_SECOND | DISK_THIRD => Some(DmaChannel::Disk),
            AUDIO0_SLOT => Some(DmaChannel::Audio(0)),
            AUDIO1_SLOT => Some(DmaChannel::Audio(1)),
            AUDIO2_SLOT => Some(DmaChannel::Audio(2)),
            AUDIO3_SLOT => Some(DmaChannel::Audio(3)),
            HPOS_SPRITE_START..=HPOS_SPRITE_END
                if (hpos % DMA_SLOT_PERIOD_CCK == DMA_ACTIVE_PHASE) =>
            {
                let sprite_num = ((hpos - HPOS_SPRITE_START) / SPRITE_SLOT_PERIOD_CCK) as u8;
                Some(DmaChannel::Sprite(sprite_num))
            }
            _ => None,
        }
    }

    /// The E0 exclusion is retained model timing; parity is relative to HPOS.
    #[inline]
    pub fn is_copper_slot(hpos: u16) -> bool {
        hpos % DMA_SLOT_PERIOD_CCK == COPPER_MEMORY_PHASE && hpos != COPPER_END_OF_LINE_EXCLUSION
    }

    /// Allocates fetches from HRM 6, Figures 6-11 and 6-12.
    pub fn bitplane_channel_for_slot(&self, hpos: u16, vpos: u16) -> Option<u8> {
        if !self.is_channel_enabled(DmaChannel::Bitplane(0))
            || !self.is_in_vertical_display(vpos)
            || !self.is_in_ddf_window(hpos)
        {
            return None;
        }
        let offset = hpos - self.ddfstrt;
        let plane = if self.is_hires() {
            HIRES_FETCH_PLANES[usize::from(offset % HIRES_FETCH_PERIOD_CCK)]
        } else {
            LORES_FETCH_PLANES[usize::from(offset % LORES_FETCH_PERIOD_CCK)]
        };
        if plane == 0 || plane > self.planecount() {
            None
        } else {
            Some(plane - 1)
        }
    }

    /// Selects one owner, then counts CPU denial regardless of who won the slot.
    pub fn step_cck(
        &mut self,
        hpos: u16,
        vpos: u16,
        disk_active: bool,
        audio_active: [bool; AUDIO_CHANNEL_COUNT],
        sprite_active: [bool; SPRITE_COUNT],
        copper_wants_bus: bool,
        blitter_wants_bus: bool,
        cpu_wants_bus: bool,
        line_ccks: u16,
    ) -> DmaChannel {
        let owner = self.select_owner(
            hpos,
            vpos,
            disk_active,
            audio_active,
            sprite_active,
            copper_wants_bus,
            blitter_wants_bus,
            cpu_wants_bus,
            line_ccks,
        );
        self.current_owner = owner;
        self.chip_ram_blocked = !matches!(owner, DmaChannel::Cpu | DmaChannel::None);
        self.cpu_starvation_counter = if cpu_wants_bus && self.chip_ram_blocked {
            self.cpu_starvation_counter
                .saturating_add(1)
                .min(BLITTER_STARVATION_YIELD_CYCLES)
        } else {
            0
        };
        owner
    }

    fn select_owner(
        &self,
        hpos: u16,
        vpos: u16,
        disk_active: bool,
        audio_active: [bool; AUDIO_CHANNEL_COUNT],
        sprite_active: [bool; SPRITE_COUNT],
        copper_wants_bus: bool,
        blitter_wants_bus: bool,
        cpu_wants_bus: bool,
        line_ccks: u16,
    ) -> DmaChannel {
        let fixed = Self::fixed_slot_for_hpos(hpos, line_ccks);
        if fixed == Some(DmaChannel::Refresh) {
            return DmaChannel::Refresh;
        }
        if self.is_dma_enabled() {
            match fixed {
                Some(DmaChannel::Disk)
                    if disk_active && self.is_channel_enabled(DmaChannel::Disk) =>
                {
                    return DmaChannel::Disk;
                }
                Some(DmaChannel::Audio(channel))
                    if audio_active[usize::from(channel)]
                        && self.is_channel_enabled(DmaChannel::Audio(channel)) =>
                {
                    return DmaChannel::Audio(channel);
                }
                _ => {}
            }
            if let Some(plane) = self.bitplane_channel_for_slot(hpos, vpos) {
                return DmaChannel::Bitplane(plane);
            }
            if let Some(DmaChannel::Sprite(sprite)) = fixed {
                if sprite_active[usize::from(sprite)]
                    && self.is_channel_enabled(DmaChannel::Sprite(sprite))
                {
                    return DmaChannel::Sprite(sprite);
                }
            }
            if copper_wants_bus
                && Self::is_copper_slot(hpos)
                && self.is_channel_enabled(DmaChannel::Copper)
            {
                return DmaChannel::Copper;
            }
            if blitter_wants_bus
                && self.is_channel_enabled(DmaChannel::Blitter)
                && (self.is_blitter_nasty()
                    || !cpu_wants_bus
                    || self.cpu_starvation_counter < BLITTER_STARVATION_YIELD_CYCLES)
            {
                return DmaChannel::Blitter;
            }
        }
        if cpu_wants_bus {
            DmaChannel::Cpu
        } else {
            DmaChannel::None
        }
    }
}
