//! Amiga 500 floppy drive controls, sense lines and host-owned image buffers.
//!
//! Models the 3.5-inch Double Density drive mechanics (80 cylinders, 2 heads,
//! Chinon F-354E), track geometry, and multi-chip hardware coordination
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
const HEAD_TRAVEL_US: u32 = 3_000;
const HEAD_SETTLE_US: u32 = 15_000;
const SIDE_READ_RECOVERY_US: u32 = 100;

/// Standard Double Density disk geometry constants
const CYLINDERS_PER_DISK: u8 = 80;
const HEADS_PER_DISK: u8 = 2;
const TRACKS_PER_DISK: usize = (CYLINDERS_PER_DISK as usize) * (HEADS_PER_DISK as usize);
pub const SECTORS_PER_TRACK: usize = 11;
pub const SECTOR_DATA_BYTES: usize = 512;
pub const FORMATTED_DISK_BYTES: usize = TRACKS_PER_DISK * SECTORS_PER_TRACK * SECTOR_DATA_BYTES; // 901,120 bytes (880 KB)

/// Writable tracks of a disk, independent of the drive that holds it.
#[derive(Debug)]
pub struct FloppyDisk {
    tracks: Vec<media::Track>,
    write_protected: bool,
}

impl FloppyDisk {
    pub fn from_adf(data: &[u8]) -> Option<Self> {
        (data.len() == FORMATTED_DISK_BYTES).then(|| Self {
            tracks: media::encode_disk(data),
            write_protected: false,
        })
    }
}

/// Individual 3.5-inch floppy disk drive (DF0: to DF3:)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyDrive {
    /// Physical attachment, independent of whether media is inserted.
    pub connected: bool,
    /// Active cylinder head position (0..79)
    pub cylinder: u8,
    /// Active head/surface (0 = Lower, 1 = Upper)
    pub side: u8,
    /// MOTOR command latched at selection.
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
    track_data: Option<Vec<media::Track>>,
    clock_hz: u32,
    spindle_speed: u32,
    spindle_fraction: u32,
    rotation_bit: u32,
    cell_phase: u32,
    index_window: bool,
    /// Destination cylinder and remaining travel in CCK.
    head_travel: Option<(u8, u32)>,
    settle_cck: u32,
    read_recovery_cck: u32,
    writing: bool,
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
            clock_hz: config::VideoStandard::Pal.color_clock_hz(),
            spindle_speed: 0,
            spindle_fraction: 0,
            rotation_bit: 0,
            cell_phase: 0,
            index_window: false,
            head_travel: None,
            settle_cck: 0,
            read_recovery_cck: 0,
            writing: false,
        }
    }
}

impl FloppyDrive {
    /// Electronic reset stops the motor, retaining physical media and head position.
    pub fn reset(&mut self) {
        self.side = LOWER_SIDE;
        self.motor_on = false;
        self.selected = false;
        self.spindle_speed = 0;
        self.head_travel = None;
        self.settle_cck = 0;
        self.read_recovery_cck = 0;
        self.writing = false;
    }

    /// Action method: sets head side (0 = lower head, 1 = upper head)
    #[inline]
    fn set_side(&mut self, side: u8) {
        if self.side != side {
            self.side = side;
            self.read_recovery_cck = self
                .read_recovery_cck
                .max(self.delay_us(SIDE_READ_RECOVERY_US));
        }
    }

    /// Action method: steps the head one cylinder inward or outward.
    /// If a disk is inserted, receiving a step pulse clears the `_CHNG` flip-flop.
    fn step_pulse(&mut self, inward: bool) {
        if self.disk_inserted {
            self.disk_change_flip_flop = false;
        }
        let next = if inward {
            (self.cylinder + 1).min(CYLINDERS_PER_DISK - 1)
        } else {
            self.cylinder.saturating_sub(1)
        };
        if next != self.cylinder && self.head_travel.is_none() {
            self.head_travel = Some((next, self.delay_us(HEAD_TRAVEL_US)));
            self.settle_cck = self.delay_us(HEAD_SETTLE_US);
        }
    }

    fn delay_us(&self, micros: u32) -> u32 {
        (u64::from(self.clock_hz) * u64::from(micros)).div_ceil(1_000_000) as u32
    }

    fn advance_head(&mut self) {
        if let Some((next, remaining)) = &mut self.head_travel {
            *remaining -= 1;
            if *remaining == 0 {
                self.cylinder = *next;
                self.head_travel = None;
            }
        } else if self.settle_cck != 0 {
            self.settle_cck -= 1;
        }
        if self.read_recovery_cck != 0 {
            self.read_recovery_cck -= 1;
        }
    }

    fn has_spindle_work(&self) -> bool {
        self.spindle_speed != 0 || (self.disk_inserted && self.motor_on) || self.writing
    }

    fn needs_media_step(&self) -> bool {
        self.connected
            && (self.track_data.is_some()
                || self.has_spindle_work()
                || self.head_travel.is_some()
                || self.settle_cck != 0
                || self.read_recovery_cck != 0)
    }

    /// Inserts a floppy disk image into the drive
    pub fn insert_disk(&mut self, data: &[u8]) -> bool {
        let Some(mut disk) = FloppyDisk::from_adf(data) else {
            return false;
        };
        disk.write_protected = self.write_protected;
        self.replace_disk(Some(disk));
        true
    }

    /// Ejects the disk from the drive
    pub fn eject_disk(&mut self) {
        self.replace_disk(None);
    }

    /// Moves the disk and its write protection tab, retaining drive mechanics.
    pub fn replace_disk(&mut self, disk: Option<FloppyDisk>) -> Option<FloppyDisk> {
        let ejected = self.track_data.take().map(|tracks| FloppyDisk {
            tracks,
            write_protected: self.write_protected,
        });
        self.disk_inserted = disk.is_some();
        self.disk_change_flip_flop = true;
        self.writing = false;
        if let Some(disk) = disk {
            self.write_protected = disk.write_protected;
            self.track_data = Some(disk.tracks);
        }
        ejected
    }

    /// Returns true if the drive head is at Track 0 (cylinder 0)
    #[inline]
    pub fn is_track0(&self) -> bool {
        self.cylinder == 0
    }

    /// Returns true if the drive is ready (spindle motor spinning and disk inserted)
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.motor_on && self.disk_inserted && self.spindle_speed == self.nominal_spindle_speed()
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
    media_output_active: bool,
}

impl Default for FloppyController {
    fn default() -> Self {
        Self::new(config::VideoStandard::Pal)
    }
}

impl FloppyController {
    pub fn new(video: config::VideoStandard) -> Self {
        Self {
            pins: FloppyPins::default(),
            drives: core::array::from_fn(|index| FloppyDrive {
                connected: index == 0,
                clock_hz: video.color_clock_hz(),
                ..FloppyDrive::default()
            }),
            prev_ciab_prb: CONTROL_RELEASED,
            media_output_active: false,
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
                    && previous & BIT_CONTROL_STEP == 0
                    && control & BIT_CONTROL_STEP != 0
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
            if self.media_output_active || self.drives.iter().any(FloppyDrive::needs_media_step) {
                self.step_media();
            }
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
        let mut attached = false;
        let mut read_high = true;
        let mut index_high = true;
        for drive in &mut self.drives {
            if drive.connected {
                attached |= drive.track_data.is_some();
                drive.advance_head();
                if !drive.has_spindle_work() {
                    continue;
                }
                let (transition, index) =
                    drive.step_rotation(!self.pins.write_enable_high, !self.pins.write_data_high);
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
