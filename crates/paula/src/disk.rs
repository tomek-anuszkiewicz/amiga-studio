//! Disk data separation, three-word FIFO, DMA count and serial write output.

use crate::register_bits::{dskbytr, intreq};
use crate::{Paula, MASK_DSKLEN_CONTROL, MASK_DSKLEN_LENGTH};
use config::mask::dmacon;
use serde::{Deserialize, Serialize};

const FIFO_WORDS: usize = config::dmal::DISK_SLOT_COUNT;
const BIT_DMA_ENABLE: u16 = 0x8000;
const BIT_FAST: u16 = 0x0100;
const BIT_MSB_SYNC: u16 = 0x0200;
const BIT_WORD_SYNC: u16 = 0x0400;
const FAST_CELL_CCK: u8 = 7;
const SLOW_CELL_CCK: u8 = 14;
const WORD_BITS: u8 = u16::BITS as u8;
const BYTE_BITS: u8 = u8::BITS as u8;
const BIT_WORD_MSB: u16 = 1 << (WORD_BITS - 1);

/// Current disk engine state, separate from drive mechanics and RAM addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskEngine {
    fifo: [u16; FIFO_WORDS],
    head: usize,
    count: usize,
    read_high: bool,
    read_phase: u8,
    read_one: bool,
    read_shift: u16,
    data_shift: u16,
    byte_bits: u8,
    word_bits: u8,
    sync_wait: bool,
    write_shift: u16,
    write_bits: u8,
    write_phase: u8,
    write_draining: bool,
    was_writing: bool,
    /// One-CCK internal source pulses, sampled after due INTREQ writes.
    pub irq_pending: u16,
}

impl Default for DiskEngine {
    fn default() -> Self {
        Self {
            fifo: [0; FIFO_WORDS],
            head: 0,
            count: 0,
            read_high: true,
            read_phase: 0,
            read_one: false,
            read_shift: 0,
            data_shift: 0,
            byte_bits: 0,
            word_bits: 0,
            sync_wait: false,
            write_shift: 0,
            write_bits: 0,
            write_phase: 0,
            write_draining: false,
            was_writing: false,
            irq_pending: 0,
        }
    }
}

impl DiskEngine {
    /// Buffered words available to the disk DMA/read or serial/write side.
    pub fn buffered_words(&self) -> usize {
        self.count
    }

    fn push(&mut self, word: u16) -> bool {
        if self.count == FIFO_WORDS {
            return false;
        }
        self.fifo[(self.head + self.count) % FIFO_WORDS] = word;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<u16> {
        if self.count == 0 {
            return None;
        }
        let word = self.fifo[self.head];
        self.head = (self.head + 1) % FIFO_WORDS;
        self.count -= 1;
        Some(word)
    }

    fn clear_transfer(&mut self) {
        self.fifo = [0; FIFO_WORDS];
        self.head = 0;
        self.count = 0;
        self.word_bits = 0;
        self.byte_bits = 0;
        self.write_bits = 0;
        self.write_phase = 0;
        self.write_draining = false;
    }
}

impl Paula {
    pub(super) fn disk_dma_enabled(&self) -> bool {
        self.dma_active && self.dma_master && self.dma_enables & dmacon::BIT_DSKEN != 0
    }

    pub(super) fn apply_disk_length(&mut self, value: u16) {
        let enabled = value & BIT_DMA_ENABLE != 0;
        let start = enabled && self.dma_armed;
        self.dsklen = value;
        self.dma_armed = enabled;
        self.dma_active = start;
        if !enabled || start {
            self.disk.clear_transfer();
            self.pins.disk_write_enable = false;
            self.pins.disk_write_high = true;
        }
        self.disk.sync_wait = start && !self.is_dsk_write() && self.adkcon & BIT_WORD_SYNC != 0;
        if start && value & MASK_DSKLEN_LENGTH == 0 {
            self.finish_disk_dma();
        }
    }

    fn finish_disk_dma(&mut self) {
        self.dma_active = false;
        self.dma_armed = false;
        self.disk.irq_pending |= intreq::BIT_DSKBLK;
    }

    fn count_disk_word(&mut self) {
        let length = self.dsklen & MASK_DSKLEN_LENGTH;
        if length != 0 {
            self.dsklen = (self.dsklen & MASK_DSKLEN_CONTROL) | length.wrapping_sub(1);
            if length == 1 {
                self.finish_disk_dma();
            }
        }
    }

    /// Supplies the early-read bus word only for a selected disk DMA transfer.
    pub fn take_disk_dma_word(&mut self) -> Option<u16> {
        if !self.disk_dma_enabled() || self.is_dsk_write() {
            return None;
        }
        let word = self.disk.pop()?;
        self.dskdat = word;
        self.count_disk_word();
        Some(word)
    }

    pub(super) fn apply_disk_data(&mut self, word: u16) {
        self.dskdat = word;
        if self.dma_active && self.is_dsk_write() && self.disk.push(word) {
            self.disk.write_draining = true;
            self.count_disk_word();
        }
    }

    pub(super) fn disk_request_count(&self) -> usize {
        if !self.disk_dma_enabled() || self.disk.sync_wait {
            return 0;
        }
        let available = if self.is_dsk_write() {
            FIFO_WORDS - self.disk.count
        } else {
            self.disk.count
        };
        available.min(usize::from(self.dsklen & MASK_DSKLEN_LENGTH))
    }

    fn receive_disk_bit(&mut self, bit: bool) {
        self.disk.read_shift = (self.disk.read_shift << 1) | u16::from(bit);
        let matched = self.disk.read_shift == self.dsksync;
        self.dskbytr &= !dskbytr::BIT_DSKBYTR_WORDEQUAL;
        if matched {
            self.dskbytr |= dskbytr::BIT_DSKBYTR_WORDEQUAL;
            self.disk.irq_pending |= intreq::BIT_DSKSYN;
        }

        let skip_leading_zero = self.adkcon & BIT_MSB_SYNC != 0 && self.disk.byte_bits == 0 && !bit;
        if !skip_leading_zero {
            self.disk.data_shift = (self.disk.data_shift << 1) | u16::from(bit);
            self.disk.byte_bits += 1;
            self.disk.word_bits += 1;
            if self.disk.byte_bits == BYTE_BITS {
                self.disk.byte_bits = 0;
                self.dskbytr = (self.dskbytr & dskbytr::BIT_DSKBYTR_WORDEQUAL)
                    | dskbytr::BIT_DSKBYTR_DSKBYT
                    | u16::from(self.disk.data_shift as u8);
            }
        }
        if matched && self.adkcon & BIT_WORD_SYNC != 0 {
            self.disk.sync_wait = false;
            self.disk.byte_bits = 0;
            self.disk.word_bits = 0;
        } else if self.disk.word_bits == WORD_BITS {
            self.disk.word_bits = 0;
            if self.disk_dma_enabled()
                && !self.disk.sync_wait
                && self.disk.count < usize::from(self.dsklen & MASK_DSKLEN_LENGTH)
            {
                self.disk.push(self.disk.data_shift);
            }
        }
    }

    fn step_disk_write(&mut self, period: u8) {
        let allowed = self.dma_master
            && self.dma_enables & dmacon::BIT_DSKEN != 0
            && self.is_dsk_write()
            && (self.dma_active || self.disk.write_draining);
        if !allowed {
            if self.disk.was_writing {
                self.pins.disk_write_enable = false;
                self.pins.disk_write_high = true;
                self.disk.was_writing = false;
            }
            return;
        }
        self.disk.was_writing = true;
        self.pins.disk_write_high = true;
        if self.disk.write_phase != 0 {
            self.disk.write_phase -= 1;
            if self.disk.write_phase != 0 {
                return;
            }
        }
        if self.disk.write_bits == 0 {
            let Some(word) = self.disk.pop() else {
                self.pins.disk_write_enable = false;
                self.disk.write_draining = false;
                return;
            };
            self.disk.write_shift = word;
            self.disk.write_bits = WORD_BITS;
        }
        self.pins.disk_write_enable = true;
        self.pins.disk_write_high = self.disk.write_shift & BIT_WORD_MSB == 0;
        self.disk.write_shift <<= 1;
        self.disk.write_bits -= 1;
        self.disk.write_phase = period;
    }

    pub(super) fn step_disk(&mut self) {
        let period = if self.adkcon & BIT_FAST != 0 {
            FAST_CELL_CCK
        } else {
            SLOW_CELL_CCK
        };
        let falling = self.disk.read_high && !self.pins.disk_read_high;
        self.disk.read_high = self.pins.disk_read_high;
        if falling {
            self.disk.read_phase = 0;
            self.disk.read_one = true;
        } else {
            self.disk.read_phase = self.disk.read_phase.wrapping_add(1) % period;
        }
        if self.disk.read_phase == period / 2 && !self.is_dsk_write() {
            let bit = core::mem::take(&mut self.disk.read_one);
            self.receive_disk_bit(bit);
        }
        self.step_disk_write(period);
    }
}
