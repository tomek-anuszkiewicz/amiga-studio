//! Amiga 500 Floppy Disk Subsystem & MFM Controller
//!
//! Models the 3.5-inch Double Density drive mechanics (80 cylinders, 2 heads,
//! Chinon FB-354 / Sony MPF-110), track geometry, and multi-chip hardware coordination
//! across CIA-A sensing ($BFE001), CIA-B mechanics ($BFD100), and Paula MFM DMA ($DFF020..$DFF024).

pub mod mfm;
pub use mfm::*;

use serde::{Deserialize, Serialize};

/// Standard Double Density disk geometry constants
pub const CYLINDERS_PER_DISK: u8 = 80;
pub const HEADS_PER_DISK: u8 = 2;
pub const TRACKS_PER_DISK: usize = (CYLINDERS_PER_DISK as usize) * (HEADS_PER_DISK as usize);
pub const SECTORS_PER_TRACK: usize = 11;
pub const SECTOR_DATA_BYTES: usize = 512;
pub const FORMATTED_DISK_BYTES: usize = TRACKS_PER_DISK * SECTORS_PER_TRACK * SECTOR_DATA_BYTES; // 901,120 bytes (880 KB)
pub const STANDARD_DSKSYN: u16 = 0x4489;
pub const DSKBYTR_DMAON: u16 = 0x4000;
pub const DSKBYTR_DISKWRITE: u16 = 0x2000;
pub const DSKBYTR_DATA_MASK: u16 = 0x90FF;

/// Individual 3.5-inch floppy disk drive (DF0: to DF3:)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyDrive {
    /// Active cylinder head position (0..79)
    pub cylinder: u8,
    /// Active head/surface (0 = Lower, 1 = Upper)
    pub side: u8,
    /// True if drive spindle motor is spinning
    pub motor_on: bool,
    /// True if drive unit is currently selected by CIA-B
    pub selected: bool,
    /// True if a disk image is currently inserted
    pub disk_inserted: bool,
    /// True if write protection tab is engaged
    pub write_protected: bool,
    /// Disk change hardware flip-flop: set when disk is removed/changed;
    /// cleared only when a disk is present AND a step pulse is received!
    pub disk_change_flip_flop: bool,
    /// Loaded 880 KB ADF sector image byte container
    #[serde(skip)]
    pub disk_data: Option<Vec<u8>>,
}

impl Default for FloppyDrive {
    fn default() -> Self {
        Self {
            cylinder: 0,
            side: 0,
            motor_on: false,
            selected: false,
            disk_inserted: false,
            write_protected: false,
            disk_change_flip_flop: true, // Power-on with no disk inserted
            disk_data: None,
        }
    }
}

impl FloppyDrive {
    /// Resets drive state to power-on default
    pub fn reset(&mut self) {
        self.cylinder = 0;
        self.side = 0;
        self.motor_on = false;
        self.selected = false;
        self.disk_inserted = false;
        self.write_protected = false;
        self.disk_change_flip_flop = true;
        self.disk_data = None;
    }

    /// Action method: sets motor power on/off
    #[inline]
    pub fn set_motor(&mut self, on: bool) {
        self.motor_on = on;
    }

    /// Action method: sets head side (0 = lower head, 1 = upper head)
    #[inline]
    pub fn set_side(&mut self, side: u8) {
        self.side = side.min(1);
    }

    /// Action method: steps the head one cylinder inward or outward.
    /// If a disk is inserted, receiving a step pulse clears the `_CHNG` flip-flop.
    pub fn step_pulse(&mut self, inward: bool) {
        if inward {
            if self.cylinder < CYLINDERS_PER_DISK - 1 {
                self.cylinder += 1;
            }
        } else if self.cylinder > 0 {
            self.cylinder -= 1;
        }

        // Hardware quirk: step pulse clears the disk change flip-flop if a disk is in the drive
        if self.disk_inserted {
            self.disk_change_flip_flop = false;
        }
    }

    /// Steps the head (legacy wrapper)
    #[inline]
    pub fn step(&mut self, inward: bool) {
        self.step_pulse(inward);
    }

    /// Inserts a floppy disk image into the drive
    pub fn insert_disk(&mut self, data: &[u8]) {
        self.disk_data = Some(data.to_vec());
        self.disk_inserted = true;
        // Inserting a disk trips the change flip-flop until stepped
        self.disk_change_flip_flop = true;
    }

    /// Ejects the disk from the drive
    pub fn eject_disk(&mut self) {
        self.disk_data = None;
        self.disk_inserted = false;
        self.disk_change_flip_flop = true;
    }

    /// Returns the current physical track index (0..159)
    #[inline]
    pub fn current_track_index(&self) -> usize {
        (self.cylinder as usize) * 2 + (self.side as usize)
    }

    /// Returns a slice to the unencoded 5632-byte track data if a disk is inserted
    #[inline]
    pub fn get_current_track_data(&self) -> Option<&[u8]> {
        let track_idx = self.current_track_index();
        let start = track_idx * (SECTORS_PER_TRACK * SECTOR_DATA_BYTES);
        let end = start + (SECTORS_PER_TRACK * SECTOR_DATA_BYTES);
        self.disk_data.as_deref().and_then(|d| {
            if end <= d.len() {
                Some(&d[start..end])
            } else {
                None
            }
        })
    }

    /// Returns true if the drive head is at Track 0 (cylinder 0)
    #[inline]
    pub fn is_track0(&self) -> bool {
        self.cylinder == 0
    }

    /// Returns true if the drive is ready (spindle motor spinning and disk inserted)
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.motor_on && self.disk_inserted
    }

    /// Returns true if the write protection tab is engaged
    #[inline]
    pub fn is_write_protected(&self) -> bool {
        self.write_protected
    }

    /// Returns true if the disk change condition is active (disk removed or unverified)
    #[inline]
    pub fn is_disk_changed(&self) -> bool {
        !self.disk_inserted || self.disk_change_flip_flop
    }
}

/// Floppy disk MFM DMA controller & Multi-Chip Bridge (Paula, CIA-A, CIA-B)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyController {
    /// 4 floppy drive units (DF0..DF3)
    pub drives: [FloppyDrive; 4],
    /// Previously latched CIA-B Port B state (to detect falling edges of _SEL0..3 and _STEP)
    pub prev_ciab_prb: u8,
    /// Disk DMA Pointer (DSKPTH / DSKPTL)
    pub dskpt: u32,
    /// Disk DMA Length register (DSKLEN: bit 15 = DMAEN, bit 14 = WRITE)
    pub dsklen: u16,
    /// True if DSKLEN write 1 has armed the DMA sequence
    pub dma_armed: bool,
    /// True if DSKLEN write 2 has activated the DMA transfer
    pub dma_active: bool,
    /// Disk DMA channel enabled via DMACON (DSKEN bit 4 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// Disk DMA data holding register (DSKDAT)
    pub dskdat: u16,
    /// Disk sync pattern register (DSKSYN, default $4489)
    pub dsksyn: u16,
    /// Disk byte and sync status register (DSKBYTR)
    pub dskbytr: u16,
    /// Audio / Disk control register (ADKCON)
    pub adkcon: u16,
    /// Disk block DMA completion interrupt strobe (DSKBLK, Level 1, bit 1)
    #[serde(default)]
    pub dskblk_irq: bool,
    /// Disk sync pattern match interrupt strobe (DSKSYN, Level 5, bit 12)
    #[serde(default)]
    pub dsksyn_irq: bool,
    /// Active raw MFM track buffer for streaming
    #[serde(skip)]
    pub mfm_track_buffer: Vec<u8>,
    /// Byte offset within the MFM track stream
    pub mfm_track_pos: usize,
    /// True when sync word has been matched during read
    pub wordsync_matched: bool,
}

impl Default for FloppyController {
    fn default() -> Self {
        Self {
            drives: [
                FloppyDrive::default(),
                FloppyDrive::default(),
                FloppyDrive::default(),
                FloppyDrive::default(),
            ],
            prev_ciab_prb: 0xFF, // All signals initially deasserted high
            dskpt: 0,
            dsklen: 0,
            dma_armed: false,
            dma_active: false,
            dma_enabled: false,
            dskdat: 0,
            dsksyn: STANDARD_DSKSYN,
            dskbytr: 0,
            adkcon: 0,
            dskblk_irq: false,
            dsksyn_irq: false,
            mfm_track_buffer: Vec::new(),
            mfm_track_pos: 0,
            wordsync_matched: false,
        }
    }
}

impl FloppyController {
    /// Creates a new floppy controller with standard sync pattern
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets floppy controller registers and drive units
    pub fn reset(&mut self) {
        self.prev_ciab_prb = 0xFF;
        self.dskpt = 0;
        self.dsklen = 0;
        self.dma_armed = false;
        self.dma_active = false;
        self.dma_enabled = false;
        self.dskdat = 0;
        self.dsksyn = STANDARD_DSKSYN;
        self.dskbytr = 0;
        self.adkcon = 0;
        self.dskblk_irq = false;
        self.dsksyn_irq = false;
        self.mfm_track_buffer.clear();
        self.mfm_track_pos = 0;
        self.wordsync_matched = false;
        for drive in &mut self.drives {
            drive.reset();
        }
    }

    /// Action method: handles CIA-B Port B ($BFD100) output writes.
    ///
    /// Drives motor latching on `_SELx` falling edge, side selection,
    /// and head stepping pulses across drives DF0: through DF3:.
    pub fn handle_ciab_port_b_write(&mut self, prb: u8) {
        let falling_edges = self.prev_ciab_prb & !prb;
        let mtr_on = (prb & 0x80) == 0; // Bit 7: _MTR active low (0 = ON)
        let side = if (prb & 0x04) != 0 { 0 } else { 1 }; // Bit 2: _SIDE (1 = Side 0, 0 = Side 1)
        let step_falling = (falling_edges & 0x01) != 0; // Bit 0: _STEP active low pulse
        let inward = (prb & 0x02) == 0; // Bit 1: _DIR (0 = inward, 1 = outward)

        for i in 0..4 {
            let sel_bit = 1 << (3 + i); // Bits 3..6: _SEL0.._SEL3 (active low)
            let is_selected = (prb & sel_bit) == 0;
            let sel_falling = (falling_edges & sel_bit) != 0;

            // Physical latching quirk: drive samples _MTR on the falling edge of its _SEL line
            if sel_falling {
                self.drives[i].set_motor(mtr_on);
            }
            self.drives[i].selected = is_selected;

            if is_selected {
                self.drives[i].set_side(side);
                if step_falling {
                    self.drives[i].step_pulse(inward);
                }
            }
        }

        self.prev_ciab_prb = prb;
    }

    /// Action method: samples input status bits for CIA-A Port A ($BFE001).
    ///
    /// Returns bits 2..5 (`_CHNG`, `_WPROT`, `_TK0`, `_RDY`) for whichever drive
    /// is currently selected, or all bits high (`$3C`) if no drive is selected.
    pub fn sample_ciaa_port_a_inputs(&self) -> u8 {
        // Find the selected drive (if multiple selected, physical bus behaves as wired-AND / lowest index)
        for drive in &self.drives {
            if drive.selected {
                let rdy = if drive.is_ready() { 0 } else { 1 }; // Bit 5: _RDY (0 = ready)
                let tk0 = if drive.is_track0() { 0 } else { 1 }; // Bit 4: _TK0 (0 = track 0)
                let wprot = if drive.is_write_protected() { 0 } else { 1 }; // Bit 3: _WPROT (0 = protected)
                let chng = if drive.is_disk_changed() { 0 } else { 1 }; // Bit 2: _CHNG (0 = changed)

                return (rdy << 5) | (tk0 << 4) | (wprot << 3) | (chng << 2);
            }
        }

        // Open collector pull-up: all sensing lines float high (1) when no drive is selected
        0x3C
    }

    /// Action method: sets Disk DMA enable state from DMACON (DSKEN bit 4 and DMAEN bit 9)
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if !enabled {
            self.dma_active = false;
        }
    }

    /// Action method: sets Disk DMA Pointer (DSKPTH / DSKPTL)
    #[inline]
    pub fn set_dskpt(&mut self, addr: u32) {
        self.dskpt = addr & 0x00FF_FFFF;
    }

    /// Action method: writes DSKLEN register following the 2-write arming sequence
    pub fn set_dsklen(&mut self, val: u16) {
        self.dsklen = val;
        let dmaen = (val & 0x8000) != 0;

        if !dmaen {
            // Writing DSKLEN with bit 15 = 0 disables and unarms DMA
            self.dma_armed = false;
            self.dma_active = false;
        } else if !self.dma_armed {
            // First write with bit 15 = 1 arms the controller
            self.dma_armed = true;
        } else {
            // Second write with bit 15 = 1 starts the DMA transfer (if enabled in DMACON)
            self.dma_active = self.dma_enabled;
            if self.dma_active {
                self.load_current_track_mfm();
                self.mfm_track_pos = 0;
                self.wordsync_matched = false;
            }
        }
    }

    /// Encodes the current track of the selected drive into raw MFM format
    pub fn load_current_track_mfm(&mut self) {
        for drive in &self.drives {
            if drive.selected {
                if let Some(track_data) = drive.get_current_track_data() {
                    let track_idx = drive.current_track_index() as u8;
                    self.mfm_track_buffer = encode_amiga_track(track_idx, track_data);
                    return;
                }
            }
        }
        // Fallback: raw stream with standard clock pattern
        self.mfm_track_buffer = vec![0xAA; RAW_MFM_TRACK_BYTES];
    }

    /// Action method: sets Disk Sync register (DSKSYNC, default $4489)
    #[inline]
    pub fn set_dsksyn(&mut self, val: u16) {
        self.dsksyn = val;
    }

    /// Action method: sets Audio/Disk Control register (ADKCON)
    #[inline]
    pub fn set_adkcon(&mut self, val: u16) {
        self.adkcon = val;
    }

    /// Returns true if disk DMA is armed in DSKLEN
    #[inline]
    pub fn is_dma_armed(&self) -> bool {
        self.dma_armed
    }

    /// Returns true if disk DMA is actively streaming words
    #[inline]
    pub fn is_dma_active(&self) -> bool {
        self.dma_active
    }

    /// Returns true if disk DMA is enabled in DSKLEN
    #[inline]
    pub fn is_dma_enabled(&self) -> bool {
        (self.dsklen & 0x8000) != 0
    }

    /// Returns true if disk DMA is configured for writing
    #[inline]
    pub fn is_write_mode(&self) -> bool {
        (self.dsklen & 0x4000) != 0
    }

    /// Returns the live 8-bit deserialized MFM data byte and composite status flags (DMAON, DISKWRITE),
    /// atomically clearing bit 15 (`DSKBYT`) per Clear-on-Read hardware semantics.
    #[inline]
    pub fn read_dskbytr(&mut self) -> u16 {
        let dmaon = if self.dma_enabled { DSKBYTR_DMAON } else { 0 };
        let diskwrite = if self.is_write_mode() {
            DSKBYTR_DISKWRITE
        } else {
            0
        };
        let val = (self.dskbytr & DSKBYTR_DATA_MASK) | dmaon | diskwrite;
        self.dskbytr &= !0x8000;
        val
    }

    /// Peeks composite DSKBYTR without clearing bit 15 (for debuggers and UI)
    #[inline]
    pub fn peek_dskbytr(&self) -> u16 {
        let dmaon = if self.dma_enabled { DSKBYTR_DMAON } else { 0 };
        let diskwrite = if self.is_write_mode() {
            DSKBYTR_DISKWRITE
        } else {
            0
        };
        (self.dskbytr & DSKBYTR_DATA_MASK) | dmaon | diskwrite
    }

    /// Advances floppy controller state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: MFM bit deserialization during active DMA
    }

    /// Advances floppy DMA streaming by 1 word slot into Chip RAM
    pub fn step_cck_ram(&mut self, chip_ram: &mut [u8]) {
        if !self.dma_active || self.mfm_track_buffer.is_empty() {
            return;
        }

        let wordsync = (self.adkcon & 0x0400) != 0;

        // If wordsync is enabled and not yet matched, scan for sync pattern
        if wordsync && !self.wordsync_matched {
            let mut found = false;
            while self.mfm_track_pos + 1 < self.mfm_track_buffer.len() {
                let word = u16::from_be_bytes([
                    self.mfm_track_buffer[self.mfm_track_pos],
                    self.mfm_track_buffer[self.mfm_track_pos + 1],
                ]);
                self.mfm_track_pos += 2;
                if word == self.dsksyn {
                    self.wordsync_matched = true;
                    self.dsksyn_irq = true;
                    self.dskbytr |= 0x1000; // Bit 12: DSKSYN matched
                    found = true;
                    break;
                }
            }
            if !found {
                self.mfm_track_pos = 0;
                return;
            }
        }

        // Fetch next MFM word from track buffer
        if self.mfm_track_pos + 1 >= self.mfm_track_buffer.len() {
            self.mfm_track_pos = 0;
        }
        let word = u16::from_be_bytes([
            self.mfm_track_buffer[self.mfm_track_pos],
            self.mfm_track_buffer[self.mfm_track_pos + 1],
        ]);
        self.mfm_track_pos += 2;

        self.dskdat = word;
        // DSKBYTR: bit 15 (DSKBYT) = 1, bit 12 (DSKSYN) retained, byte 7..0 = low byte of MFM word
        self.dskbytr = 0x8000 | (self.dskbytr & 0x1000) | ((word & 0x00FF) as u16);

        // DMA memory transfer to Chip RAM
        if !self.is_write_mode() {
            let pt = self.dskpt as usize;
            if pt + 1 < chip_ram.len() {
                chip_ram[pt] = (word >> 8) as u8;
                chip_ram[pt + 1] = (word & 0xFF) as u8;
            }
            self.dskpt = self.dskpt.wrapping_add(2);
        }

        // Decrement length counter
        let len_words = self.dsklen & 0x3FFF;
        if len_words > 1 {
            self.dsklen = (self.dsklen & 0xC000) | (len_words - 1);
        } else {
            self.dsklen &= 0xC000;
            self.dma_active = false;
            self.dskblk_irq = true; // Level 1 DSKBLK completion interrupt
        }
    }

    /// Triggers a disk block transfer finish interrupt strobe (DSKBLK, bit 1)
    #[inline]
    pub fn trigger_dskblk(&mut self) {
        self.dskblk_irq = true;
    }

    /// Polls and clears the disk block DMA completion interrupt strobe
    #[inline]
    pub fn poll_dskblk_irq(&mut self) -> bool {
        let pending = self.dskblk_irq;
        self.dskblk_irq = false;
        pending
    }

    /// Polls and clears the disk sync pattern match interrupt strobe
    #[inline]
    pub fn poll_dsksyn_irq(&mut self) -> bool {
        let pending = self.dsksyn_irq;
        self.dsksyn_irq = false;
        pending
    }
}
