//! Amiga 500 Floppy Disk Subsystem & MFM Controller
//!
//! Models the 3.5-inch Double Density drive mechanics (80 cylinders, 2 heads,
//! Chinon FB-354 / Sony MPF-110), track geometry, and Paula MFM DMA registers.

use serde::{Deserialize, Serialize};

/// Standard Double Density disk geometry constants
pub const CYLINDERS_PER_DISK: u8 = 80;
pub const HEADS_PER_DISK: u8 = 2;
pub const TRACKS_PER_DISK: usize = (CYLINDERS_PER_DISK as usize) * (HEADS_PER_DISK as usize);
pub const SECTORS_PER_TRACK: usize = 11;
pub const SECTOR_DATA_BYTES: usize = 512;
pub const FORMATTED_DISK_BYTES: usize = TRACKS_PER_DISK * SECTORS_PER_TRACK * SECTOR_DATA_BYTES; // 901,120 bytes (880 KB)
pub const STANDARD_DSKSYN: u16 = 0x4489;

/// Individual 3.5-inch floppy disk drive (DF0: to DF3:)
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
    }

    /// Steps the head one cylinder inward or outward
    #[inline]
    pub fn step(&mut self, inward: bool) {
        if inward {
            if self.cylinder < CYLINDERS_PER_DISK - 1 {
                self.cylinder += 1;
            }
        } else if self.cylinder > 0 {
            self.cylinder -= 1;
        }
    }

    /// Returns true if the drive head is at Track 0
    #[inline]
    pub fn is_track0(&self) -> bool {
        self.cylinder == 0
    }
}

/// Floppy disk MFM DMA controller (Paula registers)
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FloppyController {
    /// Disk DMA Pointer (DSKPTH / DSKPTL)
    pub dskpt: u32,
    /// Disk DMA Length register (DSKLEN: bit 15 = DMAEN, bit 14 = WRITE)
    pub dsklen: u16,
    /// Disk DMA data holding register (DSKDAT)
    pub dskdat: u16,
    /// Disk sync pattern register (DSKSYN, default $4489)
    pub dsksyn: u16,
    /// Disk byte and sync status register (DSKBYTR)
    pub dskbytr: u16,
    /// Audio / Disk control register (ADKCON)
    pub adkcon: u16,
    /// 4 floppy drive units (DF0..DF3)
    pub drives: [FloppyDrive; 4],
}

impl FloppyController {
    /// Creates a new floppy controller with standard sync pattern
    pub fn new() -> Self {
        Self {
            dsksyn: STANDARD_DSKSYN,
            ..Default::default()
        }
    }

    /// Resets floppy controller registers and drive units
    pub fn reset(&mut self) {
        self.dskpt = 0;
        self.dsklen = 0;
        self.dskdat = 0;
        self.dsksyn = STANDARD_DSKSYN;
        self.dskbytr = 0;
        self.adkcon = 0;
        for drive in &mut self.drives {
            drive.reset();
        }
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

    /// Advances floppy controller state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: MFM bit deserialization during active DMA
    }
}
