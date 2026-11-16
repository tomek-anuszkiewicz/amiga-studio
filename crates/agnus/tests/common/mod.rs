#![allow(dead_code)]

use config::chip_ram::{
    ChipRamAccess, ChipRamBudget, ChipRamOwner, ChipRamTransfer, TransferDirection,
    MASK_OCS_DMA_ADDRESS,
};

/// Explicit cycles, using the same admission state as production storage.
#[derive(Debug, Clone)]
pub struct TestChipRam {
    bytes: Vec<u8>,
    budget: ChipRamBudget,
}

impl Default for TestChipRam {
    fn default() -> Self {
        Self::from(vec![0; 512 * 1024])
    }
}

impl From<Vec<u8>> for TestChipRam {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            budget: ChipRamBudget::default(),
        }
    }
}

impl TestChipRam {
    pub fn next_cck(&mut self) {
        self.budget.begin_cck(self.budget.cck().wrapping_add(1), 0);
    }
}

impl std::ops::Deref for TestChipRam {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}
impl std::ops::DerefMut for TestChipRam {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

impl ChipRamAccess for TestChipRam {
    fn read_chip_word(&mut self, address: u32, owner: ChipRamOwner) -> u16 {
        let address = address & MASK_OCS_DMA_ADDRESS;
        self.budget.admit(ChipRamTransfer {
            owner,
            address,
            direction: TransferDirection::Read,
            bytes: 2,
        });
        let offset = address as usize & (self.bytes.len() - 1);
        u16::from_be_bytes([self.bytes[offset], self.bytes[offset + 1]])
    }
    fn write_chip_word(&mut self, address: u32, value: u16, owner: ChipRamOwner) {
        let address = address & MASK_OCS_DMA_ADDRESS;
        self.budget.admit(ChipRamTransfer {
            owner,
            address,
            direction: TransferDirection::Write,
            bytes: 2,
        });
        let offset = address as usize & (self.bytes.len() - 1);
        self.bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
}

impl PartialEq for TestChipRam {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
