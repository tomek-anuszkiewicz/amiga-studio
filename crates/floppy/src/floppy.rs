//! Amiga 500 floppy drive controls, sense lines and host-owned image buffers.
//!
//! Models the 3.5-inch Double Density drive mechanics (80 cylinders, 2 heads,
//! Chinon FB-354 / Sony MPF-110), track geometry, and multi-chip hardware coordination
//! across CIA-A sensing ($BFE001), CIA-B mechanics ($BFD100), and Paula MFM DMA ($DFF020..$DFF024).

pub mod mfm;
pub use mfm::*;

use serde::{Deserialize, Serialize};

mod pins;
pub use pins::{FloppyInputs, FloppyPins};

const CONTROL_RELEASED: u8 = u8::MAX;

const DRIVE_COUNT: usize = 4;
const BIT_CONTROL_MOTOR: u8 = 0x80;
const BIT_CONTROL_SIDE: u8 = 0x04;
const BIT_CONTROL_STEP: u8 = 0x01;
const BIT_CONTROL_DIRECTION: u8 = 0x02;
const BIT_CONTROL_SELECT_FIRST: u8 = 1 << 3;
const LOWER_SIDE: u8 = 0;
const UPPER_SIDE: u8 = 1;
const BIT_STATUS_READY_SHIFT: u8 = 5;
const BIT_STATUS_TRACK_ZERO_SHIFT: u8 = 4;
const BIT_STATUS_WRITE_PROTECT_SHIFT: u8 = 3;
const BIT_STATUS_CHANGED_SHIFT: u8 = 2;
const STATUS_RELEASED: u8 = 0x3C;

/// Standard Double Density disk geometry constants
const CYLINDERS_PER_DISK: u8 = 80;
const HEADS_PER_DISK: u8 = 2;
const TRACKS_PER_DISK: usize = (CYLINDERS_PER_DISK as usize) * (HEADS_PER_DISK as usize);
pub const SECTORS_PER_TRACK: usize = 11;
pub const SECTOR_DATA_BYTES: usize = 512;
pub const FORMATTED_DISK_BYTES: usize = TRACKS_PER_DISK * SECTORS_PER_TRACK * SECTOR_DATA_BYTES; // 901,120 bytes (880 KB)

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
            side: LOWER_SIDE,
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
        self.side = LOWER_SIDE;
        self.motor_on = false;
        self.selected = false;
        self.disk_inserted = false;
        self.write_protected = false;
        self.disk_change_flip_flop = true;
        self.disk_data = None;
    }

    /// Action method: sets head side (0 = lower head, 1 = upper head)
    #[inline]
    fn set_side(&mut self, side: u8) {
        self.side = side.min(HEADS_PER_DISK - 1);
    }

    /// Action method: steps the head one cylinder inward or outward.
    /// If a disk is inserted, receiving a step pulse clears the `_CHNG` flip-flop.
    fn step_pulse(&mut self, inward: bool) {
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
    fn is_write_protected(&self) -> bool {
        self.write_protected
    }

    /// Returns true if the disk change condition is active (disk removed or unverified)
    #[inline]
    pub fn is_disk_changed(&self) -> bool {
        !self.disk_inserted || self.disk_change_flip_flop
    }
}

/// Four drives and the captured connector boundary; disk registers live in Paula.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyController {
    pub pins: FloppyPins,
    pub drives: [FloppyDrive; DRIVE_COUNT],
    /// Last applied controls, for motor-selection and step edge detection.
    pub prev_ciab_prb: u8,
}

impl Default for FloppyController {
    fn default() -> Self {
        Self {
            pins: FloppyPins::default(),
            drives: core::array::from_fn(|_| FloppyDrive::default()),
            prev_ciab_prb: CONTROL_RELEASED,
        }
    }
}

impl FloppyController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advances control edges delivered before this CCK; no image data is streamed.
    pub fn step_cck(&mut self) {
        let current = self.pins.control_input;
        let previous = self.prev_ciab_prb;
        if current == previous {
            return;
        }
        for (index, drive) in self.drives.iter_mut().enumerate() {
            let select = BIT_CONTROL_SELECT_FIRST << index;
            drive.selected = current & select == 0;
            if drive.selected && previous & select != 0 {
                // MOTOR must already be established before the select edge.
                drive.motor_on = previous & BIT_CONTROL_MOTOR == 0;
            }
            if drive.selected {
                drive.set_side(if current & BIT_CONTROL_SIDE == 0 {
                    UPPER_SIDE
                } else {
                    LOWER_SIDE
                });
                if previous & BIT_CONTROL_STEP != 0 && current & BIT_CONTROL_STEP == 0 {
                    drive.step_pulse(current & BIT_CONTROL_DIRECTION == 0);
                }
            }
        }
        self.prev_ciab_prb = current;
    }

    /// All selected open-collector sense outputs resolve together, low dominant.
    pub fn sample_ciaa_port_a_inputs(&self) -> u8 {
        self.drives
            .iter()
            .filter(|drive| drive.selected)
            .fold(STATUS_RELEASED, |levels, drive| {
                let asserted = (u8::from(drive.is_ready()) << BIT_STATUS_READY_SHIFT)
                    | (u8::from(drive.is_track0()) << BIT_STATUS_TRACK_ZERO_SHIFT)
                    | (u8::from(drive.is_write_protected()) << BIT_STATUS_WRITE_PROTECT_SHIFT)
                    | (u8::from(drive.is_disk_changed()) << BIT_STATUS_CHANGED_SHIFT);
                levels & !asserted
            })
    }
}
