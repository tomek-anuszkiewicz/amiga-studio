//! Agnus DMA Scheduler & Bus Arbitration Engine
//!
//! Evaluates the horizontal scanline DMA slot schedule (227.5 CCKs per line),
//! channel gating via DMACON, and Chip RAM bus contention against the CPU.

use serde::{Deserialize, Serialize};

/// Custom chip DMA channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmaChannel {
    /// DRAM Refresh cycles (fixed slots CCK 0..3)
    Refresh,
    /// Floppy Disk DMA (fixed slot CCK 4)
    Disk,
    /// Audio DMA channels 0 to 3 (fixed slots CCK 5..8)
    Audio(u8),
    /// Hardware Sprite DMA channels 0 to 7 (fixed slots CCK 12..27)
    Sprite(u8),
    /// Bitplane DMA channels 1 to 6
    Bitplane(u8),
    /// Copper coprocessor DMA
    Copper,
    /// 4-channel Blitter DMA
    Blitter,
}

/// Agnus DMA scheduler and bus arbitration state
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DmaScheduler {
    /// Active DMA Control register state (DMACON / DMACONR)
    pub dmacon: u16,
}

impl DmaScheduler {
    /// Creates a new uninitialized DMA scheduler
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets DMACON to power-on default (all DMA disabled: $0000)
    pub fn reset(&mut self) {
        self.dmacon = 0;
    }

    /// Advances DMA scheduler by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Slot cycle progression
    }

    /// Updates DMACON register following SET/CLR bit 15 logic
    pub fn write_dmacon(&mut self, val: u16) {
        if (val & 0x8000) != 0 {
            // SET bits
            self.dmacon |= val & 0x7FFF;
        } else {
            // CLR bits
            self.dmacon &= !(val & 0x7FFF);
        }
    }

    /// Returns true if the master DMA enable bit (DMAEN, bit 9) is asserted
    #[inline]
    pub fn is_dma_enabled(&self) -> bool {
        (self.dmacon & 0x0200) != 0
    }

    /// Returns true if Blitter Nasty mode (BLTPRI, bit 10) is active
    #[inline]
    pub fn is_blitter_nasty(&self) -> bool {
        (self.dmacon & 0x0400) != 0
    }

    /// Checks whether a specific DMA channel is enabled in DMACON
    pub fn is_channel_enabled(&self, channel: DmaChannel) -> bool {
        if !self.is_dma_enabled() {
            return false;
        }
        match channel {
            DmaChannel::Refresh => true,
            DmaChannel::Disk => (self.dmacon & 0x0010) != 0,
            DmaChannel::Audio(ch) => (self.dmacon & (1 << (ch.min(3)))) != 0,
            DmaChannel::Sprite(_) => (self.dmacon & 0x0020) != 0,
            DmaChannel::Bitplane(_) => (self.dmacon & 0x0100) != 0,
            DmaChannel::Copper => (self.dmacon & 0x0080) != 0,
            DmaChannel::Blitter => (self.dmacon & 0x0040) != 0,
        }
    }

    /// Returns the fixed DMA channel mapped to a horizontal Color Clock slot (HPOS)
    pub fn fixed_slot_for_hpos(hpos: u16) -> Option<DmaChannel> {
        match hpos {
            0..=3 => Some(DmaChannel::Refresh),
            4 => Some(DmaChannel::Disk),
            5 => Some(DmaChannel::Audio(0)),
            6 => Some(DmaChannel::Audio(1)),
            7 => Some(DmaChannel::Audio(2)),
            8 => Some(DmaChannel::Audio(3)),
            12..=27 => {
                let sprite_num = ((hpos - 12) / 2) as u8;
                Some(DmaChannel::Sprite(sprite_num))
            }
            _ => None,
        }
    }

    /// Returns true if Chip RAM is currently blocked from CPU access
    #[inline]
    pub fn is_chip_ram_blocked(&self, hpos: u16, blitter_busy: bool) -> bool {
        if !self.is_dma_enabled() {
            return false;
        }
        // Blitter Nasty locks CPU out of Chip RAM entirely while blitting
        if blitter_busy && self.is_blitter_nasty() && self.is_channel_enabled(DmaChannel::Blitter) {
            return true;
        }
        // Check fixed priority slots
        if let Some(channel) = Self::fixed_slot_for_hpos(hpos) {
            if self.is_channel_enabled(channel) {
                return true;
            }
        }
        false
    }
}
