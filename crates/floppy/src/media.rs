//! Standard DD ADF preparation, spindle motion and magnetic transitions.

use crate::{FloppyDrive, SECTORS_PER_TRACK, SECTOR_DATA_BYTES};
use serde::{Deserialize, Serialize};

/// 500,000 raw MFM cells/s at 300 RPM: 100,000 cells per revolution.
const RAW_CELL_HZ: u32 = 500_000;
const TRACK_BYTES: usize = 12_500;
const TRACK_BITS: u32 = (TRACK_BYTES * u8::BITS as usize) as u32;
const SECTOR_TRACK_BYTES: usize = SECTORS_PER_TRACK * SECTOR_DATA_BYTES;
const BIT_BYTE_MSB: u8 = 0x80;
const INDEX_CELLS: u32 = 2_000;
const BLOCK_BYTES: usize = 16;
const SPIN_UP_MS: u32 = 500;
const COAST_DOWN_MS: u32 = 80;
const WRITE_READ_RECOVERY_US: u32 = 1_200;
// Twice the color-clock frequency makes both linear speed changes integral per CCK.
const SPINDLE_SPEED_SCALE: u32 = 2;

/// Untouched blocks store packed MFM cells.
/// A recorded block can locate a pulse at any of the 7 or 8 nominal CCK in a cell.
/// Both representations fit inline so opening the write gate never allocates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TrackBlock {
    Encoded([u8; BLOCK_BYTES]),
    Recorded([[u8; 8]; BLOCK_BYTES]),
}

impl TrackBlock {
    fn transition(&self, byte: usize, bit: usize, cell_phase: u32) -> bool {
        match self {
            Self::Encoded(bytes) => {
                cell_phase < RAW_CELL_HZ && bytes[byte] & (BIT_BYTE_MSB >> bit) != 0
            }
            Self::Recorded(samples) => samples[byte][bit] & (1 << (cell_phase / RAW_CELL_HZ)) != 0,
        }
    }

    /// Returns true when a pulse between cell boundaries changes the block representation.
    fn record(&mut self, byte: usize, bit: usize, cell_phase: u32, transition: bool) -> bool {
        let sample = 1 << (cell_phase / RAW_CELL_HZ);
        match self {
            Self::Encoded(bytes) if sample == 1 => {
                let mask = BIT_BYTE_MSB >> bit;
                if transition {
                    bytes[byte] |= mask;
                } else {
                    bytes[byte] &= !mask;
                }
                false
            }
            Self::Encoded(_) if !transition => false,
            Self::Encoded(bytes) => {
                let mut samples = bytes.map(boundary_samples);
                samples[byte][bit] |= sample;
                *self = Self::Recorded(samples);
                true
            }
            Self::Recorded(samples) => {
                if transition {
                    samples[byte][bit] |= sample;
                } else {
                    samples[byte][bit] &= !sample;
                }
                false
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track(TrackEncoding);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TrackEncoding {
    Encoded(Vec<TrackBlock>),
    Recorded(Vec<TrackBlock>),
}

impl Track {
    fn transition(&self, rotation_bit: u32, cell_phase: u32) -> bool {
        let blocks = match &self.0 {
            TrackEncoding::Encoded(_) if cell_phase >= RAW_CELL_HZ => return false,
            TrackEncoding::Encoded(blocks) | TrackEncoding::Recorded(blocks) => blocks,
        };
        let cell = magnetic_cell(rotation_bit, cell_phase);
        let byte = cell / 8;
        blocks
            .get(byte / BLOCK_BYTES)
            .is_some_and(|block| block.transition(byte % BLOCK_BYTES, cell % 8, cell_phase))
    }

    fn record(&mut self, rotation_bit: u32, cell_phase: u32, pulse: bool) {
        let blocks = match &mut self.0 {
            TrackEncoding::Encoded(blocks) | TrackEncoding::Recorded(blocks) => blocks,
        };
        let cell = magnetic_cell(rotation_bit, cell_phase);
        let byte = cell / 8;
        let Some(block) = blocks.get_mut(byte / BLOCK_BYTES) else {
            return;
        };
        if block.record(byte % BLOCK_BYTES, cell % 8, cell_phase, pulse) {
            if let TrackEncoding::Encoded(blocks) = &mut self.0 {
                self.0 = TrackEncoding::Recorded(core::mem::take(blocks));
            }
        }
    }
}

fn magnetic_cell(rotation_bit: u32, cell_phase: u32) -> usize {
    // Slot zero is the trailing boundary of the cell just passed.
    let cell = if cell_phase >= RAW_CELL_HZ {
        rotation_bit
    } else if rotation_bit == 0 {
        TRACK_BITS - 1
    } else {
        rotation_bit - 1
    };
    cell as usize
}

/// Prepare all surfaces at insertion, so selecting/seeking never allocates.
pub fn encode_disk(data: &[u8]) -> Vec<Track> {
    data.chunks_exact(SECTOR_TRACK_BYTES)
        .enumerate()
        .map(|(track, source)| encode_track(track as u8, source))
        .collect()
}

fn encode_track(track: u8, source: &[u8]) -> Track {
    let mut destination = [crate::mfm::MFM_GAP_BYTE; TRACK_BYTES];
    for (sector, payload) in source.chunks_exact(SECTOR_DATA_BYTES).enumerate() {
        let mut bytes = [0; SECTOR_DATA_BYTES];
        bytes.copy_from_slice(payload);
        let encoded = crate::mfm::encode_amiga_sector(track, sector as u8, &bytes);
        let offset = sector * encoded.len();
        destination[offset..offset + encoded.len()].copy_from_slice(&encoded);
        // Correct the first clock of the following preamble/gap at the join.
        if encoded[encoded.len() - 1] & crate::mfm::BIT_MFM_DATA_LSB != 0 {
            destination[offset + encoded.len()] &= !BIT_BYTE_MSB;
        }
        if offset != 0 && destination[offset - 1] & crate::mfm::BIT_MFM_DATA_LSB != 0 {
            destination[offset] &= !BIT_BYTE_MSB;
        }
    }
    Track(TrackEncoding::Encoded(encoded_blocks(&destination)))
}

fn boundary_samples(byte: u8) -> [u8; 8] {
    core::array::from_fn(|bit| u8::from(byte & (BIT_BYTE_MSB >> bit) != 0))
}

fn encoded_blocks(bytes: &[u8]) -> Vec<TrackBlock> {
    bytes
        .chunks(BLOCK_BYTES)
        .map(|chunk| {
            // The last block ends at the track boundary; its padding is never read.
            let mut block = [0; BLOCK_BYTES];
            block[..chunk.len()].copy_from_slice(chunk);
            TrackBlock::Encoded(block)
        })
        .collect()
}

impl FloppyDrive {
    pub(super) fn nominal_spindle_speed(&self) -> u32 {
        self.clock_hz * SPINDLE_SPEED_SCALE
    }

    /// True when the disk sweeps one nominal-speed CCK of angular distance.
    fn advance_spindle(&mut self) -> bool {
        let full_speed = self.nominal_spindle_speed();
        if self.motor_on {
            if self.spindle_speed == full_speed {
                return true;
            }
            self.spindle_speed =
                (self.spindle_speed + SPINDLE_SPEED_SCALE * 1_000 / SPIN_UP_MS).min(full_speed);
        } else {
            self.spindle_speed = self
                .spindle_speed
                .saturating_sub(SPINDLE_SPEED_SCALE * 1_000 / COAST_DOWN_MS);
        }
        self.spindle_fraction += self.spindle_speed;
        if self.spindle_fraction >= full_speed {
            self.spindle_fraction -= full_speed;
            true
        } else {
            false
        }
    }

    pub(super) fn step_rotation(&mut self, gate: bool, pulse: bool) -> (bool, bool) {
        let advanced = self.advance_spindle();
        if advanced {
            self.cell_phase += RAW_CELL_HZ;
            if self.cell_phase >= self.clock_hz {
                self.cell_phase -= self.clock_hz;
                self.rotation_bit += 1;
                if self.rotation_bit == TRACK_BITS {
                    self.rotation_bit = 0;
                    self.index_window = true;
                } else if self.rotation_bit == INDEX_CELLS {
                    self.index_window = false;
                }
            }
        }

        let readable_surface = self.selected && self.disk_inserted && self.spindle_speed != 0;
        let index = readable_surface && self.index_window;
        let accessible = readable_surface && self.settle_cck == 0 && self.head_travel.is_none();
        let writing = accessible && gate && !self.write_protected;
        if self.writing != writing {
            if !writing {
                self.read_recovery_cck = self
                    .read_recovery_cck
                    .max(self.delay_us(WRITE_READ_RECOVERY_US));
            }
            self.writing = writing;
        }
        if !accessible || !(advanced || (writing && pulse)) {
            return (false, index);
        }
        if !writing && (gate || self.read_recovery_cck != 0) {
            return (false, index);
        }

        let surface = usize::from(self.cylinder) * usize::from(crate::HEADS_PER_DISK)
            + usize::from(self.side);
        let Some(track) = self
            .track_data
            .as_mut()
            .and_then(|tracks| tracks.get_mut(surface))
        else {
            return (false, index);
        };
        if writing {
            track.record(self.rotation_bit, self.cell_phase, pulse);
            (false, index)
        } else {
            (track.transition(self.rotation_bit, self.cell_phase), index)
        }
    }
}
