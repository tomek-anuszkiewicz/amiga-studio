//! Disk direction/request decoding and actual granted RAM-word transfers.

use crate::Agnus;
use config::{chip_ram, custom_reg, dmal};

impl Agnus {
    pub(super) fn sample_disk_request(&mut self) {
        let position = self.hpos.wrapping_sub(1);
        if !(dmal::FIRST_DISK_BIT..dmal::FIRST_AUDIO_BIT).contains(&position) {
            return;
        }
        let offset = position - dmal::FIRST_DISK_BIT;
        let slot = usize::from(offset / dmal::PAIR_BITS);
        if offset % dmal::PAIR_BITS == dmal::REQUEST_BIT {
            self.disk_dma_requests[slot] = self.pins.dmal_input_high;
        } else {
            self.disk_dma_write[slot] = self.pins.dmal_input_high;
        }
    }

    fn disk_slot(&self) -> Option<usize> {
        let offset = self.hpos.checked_sub(dmal::FIRST_DISK_TRANSFER)?;
        let slot = usize::from(offset / dmal::PAIR_BITS);
        (offset % dmal::PAIR_BITS == 0 && slot < dmal::DISK_SLOT_COUNT).then_some(slot)
    }

    pub(super) fn disk_request_for_slot(&self) -> bool {
        self.pending_dma_write.is_none()
            && self.pending_dsk_dma_slot.is_none()
            && self
                .disk_slot()
                .is_some_and(|slot| self.disk_dma_requests[slot])
    }

    pub(super) fn grant_disk_word(&mut self, ram: &mut impl config::chip_ram::ChipRamAccess) {
        let Some(slot) = self.disk_slot() else {
            return;
        };
        if self.disk_dma_write[slot] {
            let word = ram.read_chip_word(self.dskpt, config::chip_ram::ChipRamOwner::Disk);
            self.pending_dma_write = Some((custom_reg::paula::REGISTER_DSKDAT, word));
            self.dskpt = self.dskpt.wrapping_add(2) & chip_ram::MASK_OCS_DMA_ADDRESS;
        } else {
            self.pending_dsk_dma_slot = Some(self.dskpt);
        }
        self.disk_dma_requests[slot] = false;
    }

    /// Completes a granted DSKDATR-to-RAM word, after the supplier accepted it.
    pub fn complete_disk_read(&mut self) {
        self.dskpt = self.dskpt.wrapping_add(2) & chip_ram::MASK_OCS_DMA_ADDRESS;
    }
}
