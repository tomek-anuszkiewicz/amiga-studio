//! Agnus DMA Scheduler & Bus Arbitration Engine
//!
//! Evaluates the horizontal scanline DMA slot schedule (227.5 CCKs per line),
//! channel gating via DMACON, dynamic Bitplane DMA cycle stealing in the DDF window,
//! 8-tier priority arbitration, Blitter Nasty vs Normal mode with 3-cycle CPU starvation yield,
//! and Chip RAM bus contention against the CPU.

use serde::{Deserialize, Serialize};

/// Custom chip DMA channels in strict priority order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmaChannel {
    /// DRAM Refresh cycles (fixed slots CCK 0..3, priority 1)
    Refresh,
    /// Floppy Disk DMA (fixed slot CCK 4, priority 2)
    Disk,
    /// Audio DMA channels 0 to 3 (fixed slots CCK 5..8, priority 3)
    Audio(u8),
    /// Bitplane DMA channels 0 to 5 (Planes 1 to 6, dynamic slots in DDF window, priority 4)
    Bitplane(u8),
    /// Hardware Sprite DMA channels 0 to 7 (fixed slots CCK 12..27, priority 5)
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
    /// Display Data Fetch Start ($092, default $0038)
    pub ddfstrt: u16,
    /// Display Data Fetch Stop ($094, default $00D0)
    pub ddfstop: u16,
    /// Display Window Start ($08E, default $2C81)
    pub diwstrt: u16,
    /// Display Window Stop ($090, default $F4C1)
    pub diwstop: u16,
    /// Consecutive memory cycles where CPU requested the bus but was held off by normal Blitter (BLTPRI=0)
    pub cpu_starvation_counter: u8,
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
            ddfstrt: 0x0038,
            ddfstop: 0x00D0,
            diwstrt: 0x2C81,
            diwstop: 0xF4C1,
            cpu_starvation_counter: 0,
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
        self.ddfstrt = 0x0038;
        self.ddfstop = 0x00D0;
        self.diwstrt = 0x2C81;
        self.diwstop = 0xF4C1;
        self.cpu_starvation_counter = 0;
        self.current_owner = DmaChannel::Cpu;
        self.chip_ram_blocked = false;
    }

    /// Sets BPLCON0 register value
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) {
        self.bplcon0 = val;
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

    /// Sets DIWSTRT register value ($08E)
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) {
        self.diwstrt = val;
    }

    /// Sets DIWSTOP register value ($090)
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) {
        self.diwstop = val;
    }

    /// Returns the active number of bitplane DMA channels (0..6)
    #[inline]
    pub fn planecount(&self) -> u8 {
        ((self.bplcon0 >> 12) & 0x07) as u8
    }

    /// Returns true if High-Resolution mode (BPLCON0 bit 15) is active
    #[inline]
    pub fn is_hires(&self) -> bool {
        (self.bplcon0 & 0x8000) != 0
    }

    /// Returns true if the horizontal position is inside the Display Data Fetch window
    #[inline]
    pub fn is_in_ddf_window(&self, hpos: u16) -> bool {
        if hpos < self.ddfstrt {
            return false;
        }
        let period = if self.is_hires() { 4 } else { 8 };
        let block_start = hpos - ((hpos.wrapping_sub(self.ddfstrt)) % period);
        block_start <= self.ddfstop
    }

    /// Returns true if the horizontal position is inside the final Display Data Fetch block of the line
    #[inline]
    pub fn is_last_bpl_block(&self, hpos: u16) -> bool {
        if hpos < self.ddfstrt {
            return false;
        }
        let period = if self.is_hires() { 4 } else { 8 };
        let block_start = hpos - ((hpos.wrapping_sub(self.ddfstrt)) % period);
        block_start == self.ddfstop
    }

    /// Returns true if the vertical position is inside active display scanlines
    #[inline]
    pub fn is_in_vertical_display(&self, vpos: u16) -> bool {
        let vstart = (self.diwstrt >> 8) & 0xFF;
        let vstop_low = (self.diwstop >> 8) & 0xFF;
        let vstop = if vstop_low < 128 {
            256 + vstop_low
        } else {
            vstop_low
        };
        if vstart == 0 && vstop_low == 0 {
            // Default active range if unconfigured: scanlines 44..300
            vpos >= 0x2C && vpos < 0x12C
        } else {
            vpos >= vstart && vpos < vstop
        }
    }

    /// Advances DMA scheduler by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Monotonic step hook
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
            DmaChannel::Cpu | DmaChannel::None => true,
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

    /// Calculates which Bitplane channel (0..5 for planes 1..6) has a DMA slot at `(hpos, vpos)`.
    ///
    /// In Low-Resolution (LoRes):
    /// - 1 to 4 planes: allocated on even cycles every 8 CCKs (phases 0, 2, 4, 6).
    ///   Odd cycles (phases 1, 3, 5, 7) remain completely free for CPU/Copper/Blitter.
    /// - 5 planes: plane 5 steals 25% of odd cycles (phase 1).
    /// - 6 planes: planes 5 & 6 steal 50% of odd cycles (phases 1 & 3).
    ///
    /// In High-Resolution (HiRes):
    /// - 1 to 4 planes: clocked at double rate. HiRes 4 planes consumes 100% of memory cycles
    ///   (phases 0..7), completely locking out the CPU during the fetch window.
    pub fn bitplane_channel_for_slot(&self, hpos: u16, vpos: u16) -> Option<u8> {
        if !self.is_channel_enabled(DmaChannel::Bitplane(0)) {
            return None;
        }
        if !self.is_in_vertical_display(vpos) || !self.is_in_ddf_window(hpos) {
            return None;
        }

        let planes = self.planecount();
        if planes == 0 {
            return None;
        }

        let phase = (hpos.wrapping_sub(self.ddfstrt)) % 8;

        if !self.is_hires() {
            // Low-Resolution (LoRes) scheduling (Canonical Agnus fetch unit sequence)
            match phase {
                1 if planes >= 4 => Some(3), // BPL4
                2 if planes >= 6 => Some(5), // BPL6
                3 if planes >= 2 => Some(1), // BPL2
                5 if planes >= 3 => Some(2), // BPL3
                6 if planes >= 5 => Some(4), // BPL5
                7 if planes >= 1 => Some(0), // BPL1
                _ => None,
            }
        } else {
            // High-Resolution (HiRes) scheduling (Canonical Agnus double fetch rate)
            match phase {
                0 | 4 if planes >= 4 => Some(3), // BPL4
                1 | 5 if planes >= 2 => Some(1), // BPL2
                2 | 6 if planes >= 3 => Some(2), // BPL3
                3 | 7 if planes >= 1 => Some(0), // BPL1
                _ => None,
            }
        }
    }

    /// Evaluates the strict 8-tier DMA bus priority hierarchy for the active Color Clock cycle:
    ///
    /// 1. Refresh (unconditional, slots 0..3)
    /// 2. Floppy Disk (slot 4, if active and enabled)
    /// 3. Audio 0..3 (slots 5..8, if active and enabled)
    /// 4. Bitplane 1..6 (DDF window, dynamic based on resolution & planecount)
    /// 5. Sprite 0..7 (slots 12..27, if enabled)
    /// 6. Copper (when instruction fetch pending)
    /// 7. Blitter (Blitter Nasty mode awards all remaining cycles; normal mode yields after 3 CPU starvation cycles)
    /// 8. CPU (awarded when no higher priority channel claimed the cycle)
    pub fn arbitrate(
        &mut self,
        hpos: u16,
        vpos: u16,
        disk_active: bool,
        audio_active: [bool; 4],
        copper_wants_bus: bool,
        blitter_wants_bus: bool,
        cpu_wants_bus: bool,
    ) -> DmaChannel {
        // Priority 1: DRAM Refresh (CCK 0..3) - Unconditional
        if hpos <= 3 {
            self.current_owner = DmaChannel::Refresh;
            self.chip_ram_blocked = true;
            return DmaChannel::Refresh;
        }

        // Priority 2: Floppy Disk DMA (CCK 4)
        if hpos == 4 && disk_active && self.is_channel_enabled(DmaChannel::Disk) {
            self.current_owner = DmaChannel::Disk;
            self.chip_ram_blocked = true;
            return DmaChannel::Disk;
        }

        // Priority 3: Audio DMA Channels 0..3 (CCK 5..8)
        if (5..=8).contains(&hpos) {
            let ch = (hpos - 5) as u8;
            if audio_active[ch as usize] && self.is_channel_enabled(DmaChannel::Audio(ch)) {
                self.current_owner = DmaChannel::Audio(ch);
                self.chip_ram_blocked = true;
                return DmaChannel::Audio(ch);
            }
        }

        // Priority 4: Dynamic Bitplane DMA (DDF window)
        if let Some(plane) = self.bitplane_channel_for_slot(hpos, vpos) {
            self.current_owner = DmaChannel::Bitplane(plane);
            self.chip_ram_blocked = true;
            return DmaChannel::Bitplane(plane);
        }

        // Priority 5: Hardware Sprite DMA Pairs (CCK 12..27)
        if (12..=27).contains(&hpos) && self.is_channel_enabled(DmaChannel::Sprite(0)) {
            let sprite_num = ((hpos - 12) / 2) as u8;
            self.current_owner = DmaChannel::Sprite(sprite_num);
            self.chip_ram_blocked = true;
            return DmaChannel::Sprite(sprite_num);
        }

        // Priority 6: Copper Coprocessor
        if copper_wants_bus && self.is_channel_enabled(DmaChannel::Copper) {
            self.current_owner = DmaChannel::Copper;
            self.chip_ram_blocked = true;
            return DmaChannel::Copper;
        }

        // Priority 7: 4-Channel DMA Blitter
        if blitter_wants_bus && self.is_channel_enabled(DmaChannel::Blitter) {
            if self.is_blitter_nasty() {
                // Blitter Nasty: locks CPU out of Chip RAM completely
                self.current_owner = DmaChannel::Blitter;
                self.chip_ram_blocked = true;
                return DmaChannel::Blitter;
            }

            // Normal Blitter mode (BLTPRI == 0): Agnus CPU starvation yield logic
            if cpu_wants_bus {
                if self.cpu_starvation_counter >= 3 {
                    // Starved for 3 consecutive memory cycles: Blitter yields 1 cycle to CPU!
                    self.cpu_starvation_counter = 0;
                    self.current_owner = DmaChannel::Cpu;
                    self.chip_ram_blocked = false;
                    return DmaChannel::Cpu;
                }
                self.cpu_starvation_counter = self.cpu_starvation_counter.saturating_add(1);
                self.current_owner = DmaChannel::Blitter;
                self.chip_ram_blocked = true;
                return DmaChannel::Blitter;
            }

            // CPU is not requesting bus; Blitter runs freely
            self.cpu_starvation_counter = 0;
            self.current_owner = DmaChannel::Blitter;
            self.chip_ram_blocked = true;
            return DmaChannel::Blitter;
        }

        // Priority 8: CPU Access (or Idle Bus)
        self.cpu_starvation_counter = 0;
        self.current_owner = DmaChannel::Cpu;
        self.chip_ram_blocked = false;
        DmaChannel::Cpu
    }

    /// Simplified check returning true if Chip RAM is currently blocked from CPU access
    #[inline]
    pub fn is_chip_ram_blocked(&self, hpos: u16, blitter_busy: bool) -> bool {
        // DRAM Refresh is unconditional
        if hpos <= 3 {
            return true;
        }
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
