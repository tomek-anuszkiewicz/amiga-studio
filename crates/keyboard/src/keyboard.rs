//! A500 keyboard controller: physical key state, type-ahead and serial protocol.

use serde::{Deserialize, Serialize};

mod pins;
mod protocol;
pub use pins::KeyboardPins;
use protocol::{BootStream, Phase, Timing};

/// HRM chapter 8 specifies a ten-keycode type-ahead buffer.
pub const KEYBOARD_BUFFER_CAPACITY: usize = 10;
pub const SCANCODE_CTRL: u8 = 0x63;
pub const SCANCODE_L_AMIGA: u8 = 0x66;
pub const SCANCODE_R_AMIGA: u8 = 0x67;
pub const SCANCODE_CAPS_LOCK: u8 = 0x62;

const BIT_KEY_RELEASE: u8 = 0x80;
const MASK_KEY_CODE: u8 = 0x7f;
const SCANCODE_LOST_SYNC: u8 = 0xf9;
const SCANCODE_BUFFER_OVERFLOW: u8 = 0xfa;
const SCANCODE_POWERUP_STREAM_START: u8 = 0xfd;
const SCANCODE_POWERUP_STREAM_END: u8 = 0xfe;
const KEY_WORD_BITS: usize = u64::BITS as usize;

/// Serializable controller, with no direct access to the receiving CIA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyboard {
    pub pins: KeyboardPins,
    pub caps_lock_active: bool,
    timing: Timing,
    phase: Phase,
    remaining: u32,
    shift: u8,
    bits_left: u8,
    active_code: Option<u8>,
    retry_code: Option<u8>,
    lost_sync_pending: bool,
    synchronizing: bool,
    startup_sync: bool,
    boot_stream: BootStream,
    held: [u64; 2],
    boot_keys: [u64; 2],
    queue: [u8; KEYBOARD_BUFFER_CAPACITY],
    queue_len: usize,
    queue_head: usize,
    overflow: bool,
    acknowledge_low: u32,
    acknowledge_guard: u8,
}

impl Keyboard {
    /// Starts power-up synchronization using the supplied motherboard CCK rate.
    pub fn new(color_clock_hz: u32) -> Self {
        let timing = Timing::new(color_clock_hz);
        Self {
            pins: KeyboardPins::default(),
            caps_lock_active: false,
            remaining: timing.portion,
            timing,
            phase: Phase::Synchronize,
            shift: 0,
            bits_left: 0,
            active_code: None,
            retry_code: None,
            lost_sync_pending: false,
            synchronizing: true,
            startup_sync: true,
            boot_stream: BootStream::Start,
            held: [0; 2],
            boot_keys: [0; 2],
            queue: [0; KEYBOARD_BUFFER_CAPACITY],
            queue_len: 0,
            queue_head: 0,
            overflow: false,
            acknowledge_low: 0,
            acknowledge_guard: 0,
        }
    }

    /// Restarts the controller and cancels active transfers and queued events.
    pub fn reset(&mut self) {
        *self = Self::new(self.timing.color_clock_hz);
    }

    /// Queues a logical keycode; rotation and active-low encoding occur on the wire.
    pub fn enqueue_scancode(&mut self, code: u8) {
        if self.queue_len == KEYBOARD_BUFFER_CAPACITY {
            self.overflow = true;
            return;
        }
        let tail = (self.queue_head + self.queue_len) % KEYBOARD_BUFFER_CAPACITY;
        self.queue[tail] = code;
        self.queue_len += 1;
    }

    fn next_queued_code(&mut self) -> Option<u8> {
        if self.queue_len != 0 {
            let code = self.queue[self.queue_head];
            self.queue_head = (self.queue_head + 1) % KEYBOARD_BUFFER_CAPACITY;
            self.queue_len -= 1;
            Some(code)
        } else if core::mem::take(&mut self.overflow) {
            Some(SCANCODE_BUFFER_OVERFLOW)
        } else {
            None
        }
    }

    fn change_key(&mut self, code: u8, down: bool) -> bool {
        let word = usize::from(code) / KEY_WORD_BITS;
        let bit = 1u64 << (usize::from(code) % KEY_WORD_BITS);
        let previous = self.held[word] & bit != 0;
        if down {
            self.held[word] |= bit;
        } else {
            self.held[word] &= !bit;
        }
        self.pins.reset_line_asserted = [SCANCODE_CTRL, SCANCODE_L_AMIGA, SCANCODE_R_AMIGA]
            .into_iter()
            .all(|key| {
                self.held[usize::from(key) / KEY_WORD_BITS]
                    & (1u64 << (usize::from(key) % KEY_WORD_BITS))
                    != 0
            });
        previous != down
    }

    /// Records a physical press; repeat generation belongs to guest software.
    pub fn key_down(&mut self, raw_code: u8) {
        let code = raw_code & MASK_KEY_CODE;
        if !self.change_key(code, true) {
            return;
        }
        if code == SCANCODE_CAPS_LOCK {
            self.caps_lock_active = !self.caps_lock_active;
            if !self.startup_sync {
                self.enqueue_scancode(
                    code | if self.caps_lock_active {
                        0
                    } else {
                        BIT_KEY_RELEASE
                    },
                );
            }
        } else if !self.startup_sync {
            self.enqueue_scancode(code);
        }
    }

    /// Records a physical release; Caps Lock reports only its toggling presses.
    pub fn key_up(&mut self, raw_code: u8) {
        let code = raw_code & MASK_KEY_CODE;
        if self.change_key(code, false) && code != SCANCODE_CAPS_LOCK && !self.startup_sync {
            self.enqueue_scancode(code | BIT_KEY_RELEASE);
        }
    }
}
