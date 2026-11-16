//! Standard DD ADF preparation and deterministic 300 RPM magnetic read stream.

use crate::{FloppyDrive, SECTORS_PER_TRACK, SECTOR_DATA_BYTES};

/// 500,000 raw MFM cells/s at 300 RPM: 100,000 cells per revolution.
const RAW_CELL_HZ: u32 = 500_000;
const TRACK_BYTES: usize = 12_500;
const TRACK_BITS: u32 = (TRACK_BYTES * u8::BITS as usize) as u32;
const SECTOR_TRACK_BYTES: usize = SECTORS_PER_TRACK * SECTOR_DATA_BYTES;
const BIT_BYTE_MSB: u8 = 0x80;

/// Prepare all surfaces at insertion, so selecting/seeking never allocates.
pub fn encode_disk(data: &[u8]) -> Vec<u8> {
    let mut tracks = vec![crate::mfm::MFM_GAP_BYTE; crate::TRACKS_PER_DISK * TRACK_BYTES];
    for (track, (source, destination)) in data
        .chunks_exact(SECTOR_TRACK_BYTES)
        .zip(tracks.chunks_exact_mut(TRACK_BYTES))
        .enumerate()
    {
        for (sector, payload) in source.chunks_exact(SECTOR_DATA_BYTES).enumerate() {
            let mut bytes = [0; SECTOR_DATA_BYTES];
            bytes.copy_from_slice(payload);
            let encoded = crate::mfm::encode_amiga_sector(track as u8, sector as u8, &bytes);
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
    }
    tracks
}

impl FloppyDrive {
    /// Returns a transition and INDEX edge; both are one-CCK active-low markers.
    /// This models sampled events, not analog pulse width or spindle acceleration.
    pub(super) fn step_rotation(&mut self, clock_hz: u32) -> (bool, bool) {
        if !self.motor_on {
            return (false, false);
        }
        self.cell_phase += RAW_CELL_HZ;
        if self.cell_phase < clock_hz {
            return (false, false);
        }
        self.cell_phase -= clock_hz;
        let track = usize::from(self.cylinder) * usize::from(crate::HEADS_PER_DISK)
            + usize::from(self.side);
        let byte = track * TRACK_BYTES + self.rotation_bit as usize / u8::BITS as usize;
        let transition = self.disk_inserted
            && self
                .track_data
                .as_ref()
                .and_then(|data| data.get(byte))
                .is_some_and(|value| value & (BIT_BYTE_MSB >> (self.rotation_bit % u8::BITS)) != 0);
        self.rotation_bit += 1;
        let index = self.rotation_bit == TRACK_BITS;
        if index {
            self.rotation_bit = 0;
        }
        (transition, index && self.disk_inserted)
    }
}
