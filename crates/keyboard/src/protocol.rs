//! HRM Appendix H: serial bits, startup synchronization and handshake timeout.

use crate::{
    Keyboard, KEY_WORD_BITS, MASK_KEY_CODE, SCANCODE_LOST_SYNC, SCANCODE_POWERUP_STREAM_END,
    SCANCODE_POWERUP_STREAM_START,
};
use serde::{Deserialize, Serialize};

const MICROSECONDS_PER_SECOND: u64 = 1_000_000;
const BIT_PORTION_US: u64 = 20;
const HANDSHAKE_MINIMUM_US: u64 = 1;
const HANDSHAKE_TIMEOUT_US: u64 = 143_000;
const SERIAL_BITS: u8 = u8::BITS as u8;
const BIT_SERIAL_MSB: u8 = 1 << (SERIAL_BITS - 1);
// Let the released device output traverse source capture and return delivery.
const ACKNOWLEDGE_GUARD_CCK: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub color_clock_hz: u32,
    pub portion: u32,
    minimum_acknowledge: u32,
    timeout: u32,
}

impl Timing {
    pub fn new(color_clock_hz: u32) -> Self {
        let clocks = |microseconds: u64| {
            (u64::from(color_clock_hz) * microseconds).div_ceil(MICROSECONDS_PER_SECOND) as u32
        };
        Self {
            color_clock_hz,
            portion: clocks(BIT_PORTION_US),
            minimum_acknowledge: clocks(HANDSHAKE_MINIMUM_US),
            timeout: clocks(HANDSHAKE_TIMEOUT_US),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Synchronize,
    Idle,
    DataSetup,
    ClockLow,
    ClockHigh,
    AwaitHandshake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootStream {
    Start,
    Keys(u8),
    End,
    Done,
}

impl Keyboard {
    fn next_code(&mut self) -> Option<u8> {
        if core::mem::take(&mut self.lost_sync_pending) {
            return Some(SCANCODE_LOST_SYNC);
        }
        if let Some(code) = self.retry_code.take() {
            return Some(code);
        }
        match self.boot_stream {
            BootStream::Start => {
                self.boot_stream = BootStream::Keys(0);
                Some(SCANCODE_POWERUP_STREAM_START)
            }
            BootStream::Keys(mut next) => {
                while next <= MASK_KEY_CODE {
                    let code = next;
                    next += 1;
                    if self.boot_keys[usize::from(code) / KEY_WORD_BITS]
                        & (1u64 << (usize::from(code) % KEY_WORD_BITS))
                        != 0
                    {
                        self.boot_stream = BootStream::Keys(next);
                        return Some(code);
                    }
                }
                self.boot_stream = BootStream::End;
                Some(SCANCODE_POWERUP_STREAM_END)
            }
            BootStream::End => {
                self.caps_lock_active = false;
                self.boot_stream = BootStream::Done;
                self.next_queued_code()
            }
            BootStream::Done => self.next_queued_code(),
        }
    }

    fn start_bits(&mut self, wire: u8, count: u8) {
        self.shift = wire;
        self.bits_left = count;
        self.pins.data_high = wire & BIT_SERIAL_MSB != 0;
        self.phase = Phase::DataSetup;
        self.remaining = self.timing.portion;
    }

    fn accept_handshake(&mut self) {
        if self.synchronizing {
            self.synchronizing = false;
            if self.startup_sync {
                self.startup_sync = false;
                self.boot_keys = self.held;
            } else {
                self.lost_sync_pending = true;
            }
        }
        self.active_code = None;
        self.phase = Phase::Idle;
        self.remaining = self.timing.portion;
    }

    fn await_handshake(&mut self) {
        if self.acknowledge_guard != 0 {
            self.acknowledge_guard -= 1;
        } else if !self.pins.host_data_high {
            self.acknowledge_low = self.acknowledge_low.saturating_add(1);
            if self.acknowledge_low >= self.timing.minimum_acknowledge {
                return; // A recognized low holds transmission until host release.
            }
        } else {
            if self.acknowledge_low >= self.timing.minimum_acknowledge {
                self.accept_handshake();
                return;
            }
            self.acknowledge_low = 0;
        }
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            if !self.synchronizing {
                if self.retry_code.is_none() {
                    self.retry_code = self.active_code;
                }
                self.active_code = None;
                self.synchronizing = true;
            }
            self.start_bits(0, 1); // Active-low logical 1, therefore release-like.
        }
    }

    /// Advances the controller by one supplied motherboard color clock.
    pub fn step_cck(&mut self) {
        if self.phase == Phase::AwaitHandshake {
            self.await_handshake();
            return;
        }
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining != 0 {
            return;
        }
        match self.phase {
            Phase::Synchronize => self.start_bits(0, 1),
            Phase::Idle => {
                if self.pins.host_data_high {
                    if let Some(code) = self.next_code() {
                        self.active_code = Some(code);
                        self.start_bits(!code.rotate_left(1), SERIAL_BITS);
                    }
                }
            }
            Phase::DataSetup => {
                self.pins.clock_high = false;
                self.phase = Phase::ClockLow;
                self.remaining = self.timing.portion;
            }
            Phase::ClockLow => {
                self.pins.clock_high = true;
                self.bits_left -= 1;
                self.phase = Phase::ClockHigh;
                self.remaining = self.timing.portion;
            }
            Phase::ClockHigh => {
                if self.bits_left == 0 {
                    self.pins.data_high = true;
                    self.phase = Phase::AwaitHandshake;
                    // Timeout starts at the last clock, before its recovery portion.
                    self.remaining = self.timing.timeout.saturating_sub(self.timing.portion);
                    self.acknowledge_low = 0;
                    self.acknowledge_guard = ACKNOWLEDGE_GUARD_CCK;
                } else {
                    self.shift <<= 1;
                    self.start_bits(self.shift, self.bits_left);
                }
            }
            Phase::AwaitHandshake => {}
        }
    }
}
