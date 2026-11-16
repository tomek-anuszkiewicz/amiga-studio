//! CPU-wide transfer admission and the two-CCK read/write cycle phases.

use physical_memory::{AddressBus, BusResult};
use serde::{Deserialize, Serialize};

const READ_CYCLE_CCK: u8 = 2;
const WRITE_TO_READ_CCK: u8 = 1;
const WRITE_AFTER_READ_CCK: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Attempt {
    address: u32,
    value: u16,
    bytes: u8,
    write: bool,
}

/// Relative phases survive snapshots and statistical clock-counter resets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusTiming {
    last_cck: Option<u64>,
    cpu_cck: u64,
    read_delay: u8,
    write_delay: u8,
    pending: Option<Attempt>,
}

impl Default for BusTiming {
    fn default() -> Self {
        Self {
            last_cck: None,
            cpu_cck: 0,
            read_delay: 0,
            write_delay: READ_CYCLE_CCK,
            pending: None,
        }
    }
}

pub struct GuardedBus<'a> {
    bus: &'a mut dyn AddressBus,
    timing: BusTiming,
    first: Option<Attempt>,
    cck: u64,
    pc: u32,
}

impl BusTiming {
    #[inline]
    pub(super) fn next_cpu_cck(&mut self) -> u64 {
        self.cpu_cck = self.cpu_cck.wrapping_add(1);
        self.cpu_cck
    }
}

impl<'a> GuardedBus<'a> {
    #[inline]
    pub(super) fn new(
        bus: &'a mut dyn AddressBus,
        mut timing: BusTiming,
        cck: u64,
        pc: u32,
    ) -> Self {
        let elapsed = timing
            .last_cck
            .map_or(1, |previous| cck.wrapping_sub(previous));
        if elapsed == 0 {
            config::fatal_emulation_error(format_args!(
                "CPU bus timing invariant: repeated CCK={cck}, PC=${pc:06X}"
            ));
        }
        let elapsed = elapsed.min(u64::from(WRITE_AFTER_READ_CCK)) as u8;
        timing.read_delay = timing.read_delay.saturating_sub(elapsed);
        timing.write_delay = timing.write_delay.saturating_sub(elapsed);
        timing.last_cck = Some(cck);
        Self {
            bus,
            timing,
            first: None,
            cck,
            pc,
        }
    }

    /// Retirement cannot discard an unaccepted transfer or its final read CCK.
    #[inline]
    pub(super) fn require_instruction_completion(&self) {
        if self.timing.pending.is_some() || self.timing.read_delay == READ_CYCLE_CCK {
            config::fatal_emulation_error(format_args!(
                "CPU bus completion invariant: CCK={}, PC=${:06X}, pending={:?}, read phase={}",
                self.cck, self.pc, self.timing.pending, self.timing.read_delay
            ));
        }
    }

    #[inline]
    pub(super) fn finish(self) -> BusTiming {
        self.timing
    }

    #[inline]
    fn admit(&mut self, operation: Attempt) {
        if self.first.is_some()
            || self
                .timing
                .pending
                .is_some_and(|pending| pending != operation)
            || (self.timing.pending.is_none()
                && if operation.write {
                    self.timing.write_delay != 0
                } else {
                    self.timing.read_delay != 0
                })
        {
            config::fatal_emulation_error(format_args!(
                "CPU bus timing/access invariant: CCK={}, PC=${:06X}, first={:?}, pending={:?}, read phase={}, write phase={}, rejected={operation:?}",
                self.cck, self.pc, self.first, self.timing.pending, self.timing.read_delay, self.timing.write_delay));
        }
        self.first = Some(operation);
    }

    #[inline]
    fn complete<T>(&mut self, result: BusResult<T>, operation: Attempt) -> BusResult<T> {
        if result.is_wait() {
            self.timing.pending = Some(operation);
        } else {
            self.timing.pending = None;
            // Reads reserve their following completion CCK. A write's preceding
            // setup CCK cannot overlap that completion. Write -> read adjacency
            // is legal because the write completes in its callback CCK.
            self.timing.read_delay = if operation.write {
                WRITE_TO_READ_CCK
            } else {
                READ_CYCLE_CCK
            };
            self.timing.write_delay = if operation.write {
                READ_CYCLE_CCK
            } else {
                WRITE_AFTER_READ_CCK
            };
        }
        result
    }
}

impl AddressBus for GuardedBus<'_> {
    #[inline]
    fn read_byte(&mut self, address: u32) -> BusResult<u8> {
        let operation = Attempt {
            address,
            value: 0,
            bytes: 1,
            write: false,
        };
        self.admit(operation);
        let result = self.bus.read_byte(address);
        self.complete(result, operation)
    }

    #[inline]
    fn read_word(&mut self, address: u32) -> BusResult<u16> {
        let operation = Attempt {
            address,
            value: 0,
            bytes: 2,
            write: false,
        };
        self.admit(operation);
        let result = self.bus.read_word(address);
        self.complete(result, operation)
    }

    #[inline]
    fn write_byte(&mut self, address: u32, value: u8) -> BusResult<()> {
        let operation = Attempt {
            address,
            value: u16::from(value),
            bytes: 1,
            write: true,
        };
        self.admit(operation);
        let result = self.bus.write_byte(address, value);
        self.complete(result, operation)
    }

    #[inline]
    fn write_tas_byte(&mut self, address: u32, value: u8) -> BusResult<()> {
        let operation = Attempt {
            address,
            value: u16::from(value),
            bytes: 1,
            write: true,
        };
        self.admit(operation);
        let result = self.bus.write_tas_byte(address, value);
        self.complete(result, operation)
    }

    #[inline]
    fn write_word(&mut self, address: u32, value: u16) -> BusResult<()> {
        let operation = Attempt {
            address,
            value,
            bytes: 2,
            write: true,
        };
        self.admit(operation);
        let result = self.bus.write_word(address, value);
        self.complete(result, operation)
    }

    #[inline]
    fn read_word_debug(&self, address: u32) -> u16 {
        self.bus.read_word_debug(address)
    }
}
