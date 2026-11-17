//! CPU-wide transfer admission and the two-CCK read/write cycle phases.

use physical_memory::{AddressBus, BusResult};
use serde::{Deserialize, Serialize};

const BUS_CYCLE_CCK: u64 = 2;
const WRITE_CALLBACK_OFFSET_CCK: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Attempt {
    address: u32,
    value: u16,
    bytes: u8,
    write: bool,
}

/// Access timestamps survive snapshots and statistical clock-counter resets.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusTiming {
    last_cck: Option<u64>,
    cpu_cck: u64,
    last_cycle_start: u64,
    pending: Option<Attempt>,
}

pub struct GuardedBus<'a> {
    bus: &'a mut dyn AddressBus,
    timing: BusTiming,
    attempted: bool,
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
        if timing.last_cck == Some(cck) {
            config::fatal_emulation_error(format_args!(
                "CPU bus timing invariant: repeated CCK={cck}, PC=${pc:06X}"
            ));
        }
        if timing.last_cck.is_none() {
            // The first read may start now; a write still needs its setup CCK.
            timing.last_cycle_start = cck.wrapping_sub(BUS_CYCLE_CCK);
        }
        timing.last_cck = Some(cck);
        Self {
            bus,
            timing,
            attempted: false,
            cck,
            pc,
        }
    }

    /// Retirement cannot discard an unaccepted transfer or its final read CCK.
    #[inline]
    pub(super) fn require_instruction_completion(&self) {
        if self.timing.pending.is_some() || self.timing.last_cycle_start == self.cck {
            config::fatal_emulation_error(format_args!(
                "CPU bus completion invariant: CCK={}, PC=${:06X}, pending={:?}, cycle start CCK={}",
                self.cck, self.pc, self.timing.pending, self.timing.last_cycle_start
            ));
        }
    }

    #[inline]
    pub(super) fn finish(self) -> BusTiming {
        self.timing
    }

    #[inline]
    fn admit(&mut self, operation: Attempt) {
        // Compare before subtracting the write callback offset: a write during
        // a read's first CCK must not underflow into an apparently long interval.
        let required = BUS_CYCLE_CCK + u64::from(operation.write);
        if self.attempted
            || self
                .timing
                .pending
                .is_some_and(|pending| pending != operation)
            || (self.timing.pending.is_none()
                && self.cck.wrapping_sub(self.timing.last_cycle_start) < required)
        {
            config::fatal_emulation_error(format_args!(
                "CPU bus timing/access invariant: CCK={}, PC=${:06X}, cycle start CCK={}, pending={:?}, rejected={operation:?}",
                self.cck, self.pc, self.timing.last_cycle_start, self.timing.pending));
        }
        self.attempted = true;
    }

    #[inline]
    fn complete<T>(&mut self, result: BusResult<T>, operation: Attempt) -> BusResult<T> {
        if result.is_wait() {
            self.timing.pending = Some(operation);
        } else {
            self.timing.pending = None;
            // Both directions use the start of their two-CCK bus cycle.
            self.timing.last_cycle_start = if operation.write {
                self.cck.wrapping_sub(WRITE_CALLBACK_OFFSET_CCK)
            } else {
                self.cck
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
