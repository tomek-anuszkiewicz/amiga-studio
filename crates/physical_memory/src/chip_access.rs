//! Counted runtime transfers and explicit side-effect-free physical inspection.

use crate::PhysicalMemory;
use config::chip_ram::{ChipRamAccess, MASK_OCS_DMA_ADDRESS};

impl PhysicalMemory {
    /// The coordinator resets its clock origin; admission stays occupied until
    /// the first genuinely new CCK, even when the old origin happened to be one.
    pub fn rebase_cck(&mut self, cck: u64) {
        self.chip_budget.rebase_cck(cck);
    }

    #[inline]
    pub fn memory_cck(&self) -> u64 {
        self.chip_budget.cck()
    }

    #[inline]
    pub fn set_cck(&mut self, cck: u64) {
        self.chip_budget.set_cck(cck);
    }

    #[inline]
    pub(crate) fn admit_cpu_chip_transfer(&mut self) {
        self.chip_budget.admit();
    }

    pub fn chip_ram_len(&self) -> usize {
        self.chip_ram.len()
    }

    #[inline]
    pub fn read_chip_byte_debug(&self, address: u32) -> u8 {
        self.chip_ram[address as usize & (self.chip_ram_len() - 1)]
    }

    #[inline]
    pub fn write_chip_byte_debug(&mut self, address: u32, value: u8) {
        let offset = address as usize & (self.chip_ram_len() - 1);
        self.chip_ram[offset] = value;
    }

    #[inline]
    pub fn read_chip_word_debug(&self, address: u32) -> u16 {
        let offset = (address & MASK_OCS_DMA_ADDRESS) as usize;
        u16::from_be_bytes([self.chip_ram[offset], self.chip_ram[offset + 1]])
    }

    #[inline]
    pub fn write_chip_word_debug(&mut self, address: u32, value: u16) {
        let offset = (address & MASK_OCS_DMA_ADDRESS) as usize;
        self.chip_ram[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
}

impl ChipRamAccess for PhysicalMemory {
    #[inline]
    fn read_chip_word(&mut self, address: u32) -> u16 {
        self.chip_budget.admit();
        self.read_chip_word_debug(address)
    }

    #[inline]
    fn write_chip_word(&mut self, address: u32, value: u16) {
        self.chip_budget.admit();
        self.write_chip_word_debug(address, value);
    }
}
