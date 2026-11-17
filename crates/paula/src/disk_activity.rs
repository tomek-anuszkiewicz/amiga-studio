//! Presentation observations recorded at accepted disk source events.
//!
//! Recipient eligibility is supplied by the motherboard, separately from pins
//! and disk engine state. These values never affect emulated execution.

use config::FLOPPY_DRIVE_COUNT;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskActivity {
    pub sequences: [u64; FLOPPY_DRIVE_COUNT],
    recipients: [bool; FLOPPY_DRIVE_COUNT],
}

impl DiskActivity {
    /// Establishes eligibility for subsequent source events, without an event.
    pub fn set_recipients(&mut self, recipients: [bool; FLOPPY_DRIVE_COUNT]) {
        self.recipients = recipients;
    }

    pub fn record_control(&mut self, drive: usize) {
        self.sequences[drive] = self.sequences[drive].wrapping_add(1);
    }

    pub(super) fn record_transfer(&mut self) {
        for (sequence, recipient) in self.sequences.iter_mut().zip(self.recipients) {
            if recipient {
                *sequence = sequence.wrapping_add(1);
            }
        }
    }

    pub(super) fn reset(&mut self) {
        self.sequences = [0; FLOPPY_DRIVE_COUNT];
        self.recipients = [false; FLOPPY_DRIVE_COUNT];
    }
}
