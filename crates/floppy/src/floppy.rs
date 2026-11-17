//! Amiga 500 floppy drive controls, sense lines and host-owned image buffers.
//!
//! Models the 3.5-inch Double Density drive mechanics (80 cylinders, 2 heads,
//! Chinon FB-354 / Sony MPF-110), track geometry, and multi-chip hardware coordination
//! across CIA-A sensing ($BFE001), CIA-B mechanics ($BFD100), and Paula MFM DMA ($DFF020..$DFF024).

pub mod mfm;
pub use mfm::*;
mod media;

use config::FLOPPY_DRIVE_COUNT;
use serde::{Deserialize, Serialize};

mod pins;
pub use pins::{FloppyInputs, FloppyPins};

const CONTROL_RELEASED: u8 = u8::MAX;

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
    /// Physical attachment, independent of whether media is inserted.
    pub connected: bool,
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
    /// Prepared circular raw tracks, including usable media in serialized state.
    track_data: Option<Vec<u8>>,
    rotation_bit: u32,
    cell_phase: u32,
}

impl Default for FloppyDrive {
    fn default() -> Self {
        Self {
            connected: true,
            cylinder: 0,
            side: LOWER_SIDE,
            motor_on: false,
            selected: false,
            disk_inserted: false,
            write_protected: false,
            disk_change_flip_flop: true, // Power-on with no disk inserted
            track_data: None,
            rotation_bit: 0,
            cell_phase: 0,
        }
    }
}

impl FloppyDrive {
    /// Electronic reset stops the motor, retaining physical media and head position.
    pub fn reset(&mut self) {
        self.side = LOWER_SIDE;
        self.motor_on = false;
        self.selected = false;
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
    pub fn insert_disk(&mut self, data: &[u8]) -> bool {
        if data.len() != FORMATTED_DISK_BYTES {
            return false;
        }
        self.track_data = Some(media::encode_disk(data));
        self.disk_inserted = true;
        // Inserting a disk trips the change flip-flop until stepped
        self.disk_change_flip_flop = true;
        true
    }

    /// Ejects the disk from the drive
    pub fn eject_disk(&mut self) {
        self.track_data = None;
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

/// Decoded connector controls; motor latching uses the level before selection.
#[derive(Debug, Clone, Copy)]
pub struct DriveControl {
    pub selected: bool,
    pub motor_on: bool,
    pub side: u8,
    pub step: Option<bool>,
}

/// Four drives and the captured connector boundary; disk registers live in Paula.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyController {
    pub pins: FloppyPins,
    pub drives: [FloppyDrive; FLOPPY_DRIVE_COUNT],
    /// Last applied controls, for motor-selection and step edge detection.
    pub prev_ciab_prb: u8,
    clock_hz: u32,
    media_output_active: bool,
}

impl Default for FloppyController {
    fn default() -> Self {
        Self {
            pins: FloppyPins::default(),
            drives: core::array::from_fn(|index| FloppyDrive {
                connected: index == 0,
                ..FloppyDrive::default()
            }),
            prev_ciab_prb: CONTROL_RELEASED,
            clock_hz: config::VideoStandard::Pal.color_clock_hz(),
            media_output_active: false,
        }
    }
}

impl FloppyController {
    pub fn new(video: config::VideoStandard) -> Self {
        Self {
            clock_hz: video.color_clock_hz(),
            ..Self::default()
        }
    }

    /// Configure physical attachments before launch; DF0 is the internal drive.
    pub fn set_connections(&mut self, connected: [bool; FLOPPY_DRIVE_COUNT]) -> Result<(), String> {
        if !connected[0] {
            return Err("The internal DF0 drive must remain connected".into());
        }
        for (drive, connected) in self.drives.iter_mut().zip(connected) {
            drive.connected = connected;
            if !connected {
                drive.reset();
                drive.eject_disk();
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.pins = FloppyPins::default();
        self.prev_ciab_prb = CONTROL_RELEASED;
        self.media_output_active = false;
        for drive in &mut self.drives {
            drive.reset();
        }
    }

    /// Projects the next connector control delivery from the existing mechanical state.
    /// The motherboard can inspect these transitions without applying them early.
    pub fn control_states(&self, control: u8) -> [DriveControl; FLOPPY_DRIVE_COUNT] {
        let previous = self.prev_ciab_prb;
        core::array::from_fn(|index| {
            let drive = &self.drives[index];
            let select = BIT_CONTROL_SELECT_FIRST << index;
            let selected = drive.connected && control & select == 0;
            DriveControl {
                selected,
                motor_on: if selected && previous & select != 0 {
                    previous & BIT_CONTROL_MOTOR == 0
                } else {
                    drive.motor_on
                },
                side: if control & BIT_CONTROL_SIDE == 0 {
                    UPPER_SIDE
                } else {
                    LOWER_SIDE
                },
                step: if selected
                    && previous & BIT_CONTROL_STEP != 0
                    && control & BIT_CONTROL_STEP == 0
                {
                    Some(control & BIT_CONTROL_DIRECTION == 0)
                } else {
                    None
                },
            }
        })
    }

    /// Advances control edges and every running spindle, including deselected drives.
    pub fn step_cck(&mut self) {
        let current = self.pins.control_input;
        let previous = self.prev_ciab_prb;
        if current == previous {
            self.step_media();
            return;
        }
        let controls = self.control_states(current);
        for (drive, control) in self.drives.iter_mut().zip(controls) {
            drive.selected = control.selected;
            drive.motor_on = control.motor_on;
            if control.selected {
                drive.set_side(control.side);
                if let Some(inward) = control.step {
                    drive.step_pulse(inward);
                }
            }
        }
        self.prev_ciab_prb = current;
        self.step_media();
    }

    fn step_media(&mut self) {
        // Unattached deterministic pin fixtures retain control of the connector.
        let attached = self.drives.iter().any(|drive| drive.track_data.is_some());
        let mut read_high = true;
        let mut index_high = true;
        for drive in &mut self.drives {
            let (transition, index) = drive.step_rotation(self.clock_hz);
            if drive.connected && drive.selected {
                read_high &= !transition;
                index_high &= !index;
            }
        }
        if attached || self.media_output_active {
            self.pins.read_data_high = read_high;
            self.pins.index_high = index_high;
        }
        self.media_output_active = attached;
    }

    /// All selected open-collector sense outputs resolve together, low dominant.
    pub fn sample_ciaa_port_a_inputs(&self) -> u8 {
        self.drives
            .iter()
            .enumerate()
            .filter(|(_, drive)| drive.connected && drive.selected)
            .fold(STATUS_RELEASED, |levels, (index, drive)| {
                // Every bit of the supported external DD drive ID is one,
                // active low on READY while the motor is off (TRM section 1).
                let ready = drive.is_ready() || (index != 0 && !drive.motor_on);
                let asserted = (u8::from(ready) << BIT_STATUS_READY_SHIFT)
                    | (u8::from(drive.is_track0()) << BIT_STATUS_TRACK_ZERO_SHIFT)
                    | (u8::from(drive.is_write_protected()) << BIT_STATUS_WRITE_PROTECT_SHIFT)
                    | (u8::from(drive.is_disk_changed()) << BIT_STATUS_CHANGED_SHIFT);
                levels & !asserted
            })
    }
}
