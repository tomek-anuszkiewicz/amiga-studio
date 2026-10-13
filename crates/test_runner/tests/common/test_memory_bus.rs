//! Synthetic Test Memory Bus for CPU SingleStepTests
//!
//! Provides an isolated memory space with configurable unmapped defaults,
//! cycle-exact transaction recording without authentic
//! Amiga custom chip or ROM overhead.
//!
//! Supports two backend storage models:
//! - `TestMemoryStorage::Sparse`: `HashMap<u32, u8>` for flexible unmapped byte defaults.
//! - `TestMemoryStorage::Flat`: 16 MB pre-allocated buffer with dirty-address tracking for
//!   ultra-fast zero-allocation test loops ($O(1)$ memory access and $O(K)$ clean resets).

use super::transactions::{BusAccessSize, RecordedTransaction};
use physical_memory::{AddressBus, BusResult};
use std::collections::HashMap;

/// 24-bit address space size (16 MB = 16,777,216 bytes)
pub const FLAT_TEST_RAM_SIZE: usize = 16 * 1024 * 1024;

/// Internal storage model for TestMemoryBus
#[derive(Debug)]
pub enum TestMemoryStorage {
    /// Sparse hash map representation (useful for arbitrary unmapped byte defaults)
    Sparse(HashMap<u32, u8>),
    /// Flat 16 MB pre-allocated buffer with dirty-address tracking for O(1) accesses and O(K) reset
    Flat { mem: Box<[u8]>, dirty: Vec<u32> },
}

/// Lightweight test memory bus for synthetic CPU test runners and verification harnesses
#[derive(Debug)]
pub struct TestMemoryBus {
    /// Memory storage engine (Sparse or Flat)
    pub storage: TestMemoryStorage,
    /// Optional cycle-exact transaction log for instruction verification
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Elapsed CPU clocks at the current CCK callback, set by the SingleStep driver.
    pub callback_clock: u32,
    /// Value returned when reading unpopulated memory (defaults to 0xFF, configurable to 0x00)
    pub unmapped_byte: u8,
}

impl Default for TestMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl TestMemoryBus {
    /// Creates a new test memory bus with sparse 24-bit test RAM and default 0xFF unmapped byte
    pub fn new() -> Self {
        Self {
            storage: TestMemoryStorage::Sparse(HashMap::with_capacity(64)),
            transaction_log: None,
            callback_clock: 0,
            unmapped_byte: physical_memory::OPEN_BUS_BYTE,
        }
    }

    /// Creates a high-performance flat 16 MB test memory bus with 0x00 unmapped byte and dirty tracking
    pub fn new_flat() -> Self {
        Self {
            storage: TestMemoryStorage::Flat {
                mem: vec![0u8; FLAT_TEST_RAM_SIZE].into_boxed_slice(),
                dirty: Vec::with_capacity(128),
            },
            transaction_log: None,
            callback_clock: 0,
            unmapped_byte: 0x00,
        }
    }

    /// Clears memory and test state for the next test iteration (O(K) reset for Flat storage)
    pub fn clear(&mut self) {
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                for addr in dirty.drain(..) {
                    mem[(addr as usize) & 0x00FF_FFFF] = self.unmapped_byte;
                }
            }
            TestMemoryStorage::Sparse(map) => {
                map.clear();
            }
        }
        if let Some(log) = &mut self.transaction_log {
            log.clear();
        }
        self.callback_clock = 0;
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

    /// Loads a sequence of [address, byte] tuples into physical test memory
    pub fn load_test_ram(&mut self, entries: &[[u32; 2]]) {
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                for entry in entries {
                    let addr = entry[0];
                    let val = (entry[1] & 0xFF) as u8;
                    let idx = (addr as usize) & 0x00FF_FFFF;
                    mem[idx] = val;
                    dirty.push(addr);
                }
            }
            TestMemoryStorage::Sparse(map) => {
                for entry in entries {
                    let addr = entry[0];
                    let val = (entry[1] & 0xFF) as u8;
                    map.insert(addr, val);
                }
            }
        }
    }

    /// Inverts populated bytes and the unmapped default for memory mutation variants
    pub fn invert_test_memory(&mut self) {
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                for &addr in dirty.iter() {
                    let idx = (addr as usize) & 0x00FF_FFFF;
                    mem[idx] = !mem[idx];
                }
            }
            TestMemoryStorage::Sparse(map) => {
                for val in map.values_mut() {
                    *val = !*val;
                }
            }
        }
        self.unmapped_byte = !self.unmapped_byte;
    }

    /// Side-effect-free byte read for debugger inspection and test result assertions
    #[inline]
    pub fn read_byte_debug(&self, addr: u32) -> u8 {
        match &self.storage {
            TestMemoryStorage::Flat { mem, .. } => mem[(addr as usize) & 0x00FF_FFFF],
            TestMemoryStorage::Sparse(map) => *map.get(&addr).unwrap_or(&self.unmapped_byte),
        }
    }

    /// Side-effect-free word read for debugger inspection and test result assertions
    #[inline]
    pub fn read_word_debug(&self, addr: u32) -> u16 {
        let (b0, b1) = match &self.storage {
            TestMemoryStorage::Flat { mem, .. } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                let idx_next = (addr.wrapping_add(1) as usize) & 0x00FF_FFFF;
                (mem[idx], mem[idx_next])
            }
            TestMemoryStorage::Sparse(map) => (
                *map.get(&addr).unwrap_or(&self.unmapped_byte),
                *map.get(&(addr.wrapping_add(1)))
                    .unwrap_or(&self.unmapped_byte),
            ),
        };
        u16::from_be_bytes([b0, b1])
    }

    /// Side-effect-free byte write for test setup
    #[inline]
    pub fn write_byte_debug(&mut self, addr: u32, val: u8) {
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                mem[idx] = val;
                dirty.push(addr);
            }
            TestMemoryStorage::Sparse(map) => {
                map.insert(addr, val);
            }
        }
    }

    /// Side-effect-free word write for test setup
    #[inline]
    pub fn write_word_debug(&mut self, addr: u32, val: u16) {
        let bytes = val.to_be_bytes();
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                let idx_next = (addr.wrapping_add(1) as usize) & 0x00FF_FFFF;
                mem[idx] = bytes[0];
                mem[idx_next] = bytes[1];
                dirty.push(addr);
                dirty.push(addr.wrapping_add(1));
            }
            TestMemoryStorage::Sparse(map) => {
                map.insert(addr, bytes[0]);
                map.insert(addr.wrapping_add(1), bytes[1]);
            }
        }
    }
}

impl AddressBus for TestMemoryBus {
    #[inline]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        let val = match &self.storage {
            TestMemoryStorage::Flat { mem, .. } => mem[(addr as usize) & 0x00FF_FFFF],
            TestMemoryStorage::Sparse(map) => *map.get(&addr).unwrap_or(&self.unmapped_byte),
        };
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                clock: self.callback_clock,
                is_read: true,
                addr,
                size: BusAccessSize::Byte,
                data: val as u16,
            });
        }
        BusResult::Ready(val)
    }

    #[inline]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        let (b0, b1) = match &self.storage {
            TestMemoryStorage::Flat { mem, .. } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                let idx_next = (addr.wrapping_add(1) as usize) & 0x00FF_FFFF;
                (mem[idx], mem[idx_next])
            }
            TestMemoryStorage::Sparse(map) => (
                *map.get(&addr).unwrap_or(&self.unmapped_byte),
                *map.get(&(addr.wrapping_add(1)))
                    .unwrap_or(&self.unmapped_byte),
            ),
        };
        let val = u16::from_be_bytes([b0, b1]);
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                clock: self.callback_clock,
                is_read: true,
                addr,
                size: BusAccessSize::Word,
                data: val,
            });
        }
        BusResult::Ready(val)
    }

    #[inline]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                mem[idx] = val;
                dirty.push(addr);
            }
            TestMemoryStorage::Sparse(map) => {
                map.insert(addr, val);
            }
        }
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                clock: self.callback_clock,
                is_read: false,
                addr,
                size: BusAccessSize::Byte,
                data: val as u16,
            });
        }
        BusResult::Ready(())
    }

    #[inline]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let bytes = val.to_be_bytes();
        match &mut self.storage {
            TestMemoryStorage::Flat { mem, dirty } => {
                let idx = (addr as usize) & 0x00FF_FFFF;
                let idx_next = (addr.wrapping_add(1) as usize) & 0x00FF_FFFF;
                mem[idx] = bytes[0];
                mem[idx_next] = bytes[1];
                dirty.push(addr);
                dirty.push(addr.wrapping_add(1));
            }
            TestMemoryStorage::Sparse(map) => {
                map.insert(addr, bytes[0]);
                map.insert(addr.wrapping_add(1), bytes[1]);
            }
        }
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction {
                clock: self.callback_clock,
                is_read: false,
                addr,
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
