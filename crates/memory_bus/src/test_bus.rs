//! Synthetic Test Memory Bus for CPU SingleStepTests & Contention Verification
//!
//! Provides an isolated, sparse 24-bit memory space with configurable unmapped defaults,
//! cycle-exact transaction recording, and DMA stall simulation without authentic
//! Amiga custom chip or ROM overhead.

use super::{
    arbitration::{BusAccessSize, BusResult},
    bus_trait::{AddressBus, RecordedTransaction},
};
use std::collections::HashMap;

/// Lightweight test memory bus for synthetic CPU test runners and verification harnesses
#[derive(Debug, Clone, Default)]
pub struct TestMemoryBus {
    /// Sparse flat 24-bit test RAM mapping
    pub test_memory: Option<HashMap<u32, u8>>,
    /// Optional cycle-exact transaction log for instruction verification
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Value returned when reading unpopulated memory (defaults to 0xFF, configurable to 0x00)
    pub unmapped_byte: u8,
    /// Flag simulating custom chip (Agnus DMA) contention stalling CPU Chip RAM access
    pub chip_ram_blocked: bool,
}

impl TestMemoryBus {
    /// Creates a new test memory bus with sparse 24-bit test RAM and default 0xFF unmapped byte
    pub fn new() -> Self {
        Self {
            test_memory: Some(HashMap::with_capacity(64)),
            transaction_log: None,
            unmapped_byte: 0xFF,
            chip_ram_blocked: false,
        }
    }

    /// Enables or disables transaction recording
    #[inline]
    pub fn enable_transaction_recording(&mut self, enabled: bool) {
        if enabled {
            self.transaction_log = Some(Vec::new());
        } else {
            self.transaction_log = None;
        }
    }

    /// Returns a slice of recorded transactions, if recording is enabled
    #[inline]
    pub fn recorded_transactions(&self) -> Option<&[RecordedTransaction]> {
        self.transaction_log.as_deref()
    }

    /// Returns the default byte returned when reading unpopulated memory
    #[inline]
    pub fn unmapped_byte(&self) -> u8 {
        self.unmapped_byte
    }

    /// Sets the byte returned when reading unpopulated memory (0x00 for flat SingleStepTests)
    #[inline]
    pub fn set_unmapped_byte(&mut self, val: u8) {
        self.unmapped_byte = val;
    }

    /// Simulates Agnus DMA locking Chip RAM (triggering WaitState on subsequent accesses)
    #[inline]
    pub fn lock_chip_ram(&mut self) {
        self.chip_ram_blocked = true;
    }

    /// Releases Agnus DMA lock on Chip RAM
    #[inline]
    pub fn unlock_chip_ram(&mut self) {
        self.chip_ram_blocked = false;
    }

    /// Returns true if Chip RAM is blocked by DMA contention
    #[inline]
    pub fn is_chip_ram_locked(&self) -> bool {
        self.chip_ram_blocked
    }

    /// Loads a sequence of [address, byte] tuples into physical test memory
    pub fn load_test_ram(&mut self, entries: &[[u32; 2]]) {
        if let Some(test_mem) = &mut self.test_memory {
            for entry in entries {
                let addr = entry[0] & 0x00FF_FFFF;
                let val = (entry[1] & 0xFF) as u8;
                test_mem.insert(addr, val);
            }
        }
    }

    /// Inverts all bytes in test memory (used by DMA contention tests to detect unauthorized bus writes)
    pub fn invert_test_memory(&mut self) {
        if let Some(test_mem) = &mut self.test_memory {
            for val in test_mem.values_mut() {
                *val = !*val;
            }
        }
        self.unmapped_byte = !self.unmapped_byte;
    }

    /// Side-effect-free byte read for debugger inspection and test result assertions
    #[inline]
    pub fn read_byte_debug(&self, addr: u32) -> u8 {
        let addr_masked = addr & 0x00FF_FFFF;
        match &self.test_memory {
            Some(map) => *map.get(&addr_masked).unwrap_or(&self.unmapped_byte),
            None => self.unmapped_byte,
        }
    }

    /// Side-effect-free word read for debugger inspection and test result assertions
    #[inline]
    pub fn read_word_debug(&self, addr: u32) -> u16 {
        let addr_masked = addr & 0x00FF_FFFF;
        let (b0, b1) = match &self.test_memory {
            Some(map) => (
                *map.get(&addr_masked).unwrap_or(&self.unmapped_byte),
                *map.get(&(addr_masked.wrapping_add(1) & 0x00FF_FFFF))
                    .unwrap_or(&self.unmapped_byte),
            ),
            None => (self.unmapped_byte, self.unmapped_byte),
        };
        u16::from_be_bytes([b0, b1])
    }

    /// Side-effect-free byte write for test setup
    #[inline]
    pub fn write_byte_debug(&mut self, addr: u32, val: u8) {
        let addr_masked = addr & 0x00FF_FFFF;
        if let Some(map) = &mut self.test_memory {
            map.insert(addr_masked, val);
        }
    }

    /// Side-effect-free word write for test setup
    #[inline]
    pub fn write_word_debug(&mut self, addr: u32, val: u16) {
        let addr_masked = addr & 0x00FF_FFFF;
        let bytes = val.to_be_bytes();
        if let Some(map) = &mut self.test_memory {
            map.insert(addr_masked, bytes[0]);
            map.insert(addr_masked.wrapping_add(1) & 0x00FF_FFFF, bytes[1]);
        }
    }
}

impl AddressBus for TestMemoryBus {
    #[inline]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        if self.chip_ram_blocked {
            return BusResult::WaitState;
        }
        let addr_masked = addr & 0x00FF_FFFF;
        let val = match &self.test_memory {
            Some(map) => *map.get(&addr_masked).unwrap_or(&self.unmapped_byte),
            None => self.unmapped_byte,
        };
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                is_read: true,
                addr: addr_masked,
                size: BusAccessSize::Byte,
                data: val as u16,
            });
        }
        BusResult::Ready(val)
    }

    #[inline]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        if self.chip_ram_blocked {
            return BusResult::WaitState;
        }
        let addr_masked = addr & 0x00FF_FFFF;
        let (b0, b1) = match &self.test_memory {
            Some(map) => (
                *map.get(&addr_masked).unwrap_or(&self.unmapped_byte),
                *map.get(&(addr_masked.wrapping_add(1) & 0x00FF_FFFF))
                    .unwrap_or(&self.unmapped_byte),
            ),
            None => (self.unmapped_byte, self.unmapped_byte),
        };
        let val = u16::from_be_bytes([b0, b1]);
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                is_read: true,
                addr: addr_masked,
                size: BusAccessSize::Word,
                data: val,
            });
        }
        BusResult::Ready(val)
    }

    #[inline]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        if self.chip_ram_blocked {
            return BusResult::WaitState;
        }
        let addr_masked = addr & 0x00FF_FFFF;
        if let Some(map) = &mut self.test_memory {
            map.insert(addr_masked, val);
        }
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                is_read: false,
                addr: addr_masked,
                size: BusAccessSize::Byte,
                data: val as u16,
            });
        }
        BusResult::Ready(())
    }

    #[inline]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        if self.chip_ram_blocked {
            return BusResult::WaitState;
        }
        let addr_masked = addr & 0x00FF_FFFF;
        let bytes = val.to_be_bytes();
        if let Some(map) = &mut self.test_memory {
            map.insert(addr_masked, bytes[0]);
            map.insert(addr_masked.wrapping_add(1) & 0x00FF_FFFF, bytes[1]);
        }
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                is_read: false,
                addr: addr_masked,
                size: BusAccessSize::Word,
                data: val,
            });
        }
        BusResult::Ready(())
    }

    #[inline]
    fn read_word_debug(&self, addr: u32) -> u16 {
        TestMemoryBus::read_word_debug(self, addr)
    }
}
