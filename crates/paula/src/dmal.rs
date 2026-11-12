//! Line-anchored DMAL generation from Paula-owned pending channel requests.

use crate::Paula;
use config::{dmal, AUDIO_CHANNEL_COUNT};

impl Paula {
    pub(super) fn capture_dma_requests(&mut self) {
        self.sync.dmal_frame = 0;
        let disk_words = self.disk_request_count();
        // Received words settle into the last FIFO stages before DMAL capture.
        let first_slot = if self.is_dsk_write() {
            0
        } else {
            dmal::DISK_SLOT_COUNT - disk_words
        };
        for slot in first_slot..first_slot + disk_words {
            let bit = dmal::FIRST_DISK_BIT + slot as u16 * dmal::PAIR_BITS;
            self.sync.dmal_frame |= 1 << (bit + dmal::REQUEST_BIT);
            if self.is_dsk_write() {
                self.sync.dmal_frame |= 1 << bit;
            }
        }
        for channel in 0..AUDIO_CHANNEL_COUNT {
            let audio = &self.audio.channels[channel];
            if audio.dma_enabled && audio.dma_request {
                let bit = dmal::FIRST_AUDIO_BIT + channel as u16 * dmal::PAIR_BITS;
                self.sync.dmal_frame |= 1 << (bit + dmal::REQUEST_BIT);
                if audio.restart_request {
                    self.sync.dmal_frame |= 1 << bit;
                }
            }
        }
    }

    pub(super) fn drive_dmal(&mut self) {
        let position = self
            .sync
            .cck_since_horizontal
            .saturating_add(dmal::HORIZONTAL_CAPTURE_POSITION);
        self.pins.dmal_output_high =
            position < dmal::FRAME_BITS && self.sync.dmal_frame & (1 << position) != 0;
    }
}
