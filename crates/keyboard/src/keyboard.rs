//! Amiga Keyboard Subsystem (MOS 6500/1 Microcontroller)
//!
//! Models keyboard matrix scanning, 10-key type-ahead buffer,
//! bidirectional serial communications to CIA-A SP/CNT, and Ctrl-Amiga-Amiga reset.

use serde::{Deserialize, Serialize};

/// Maximum capacity of the physical keyboard type-ahead FIFO queue
pub const KEYBOARD_BUFFER_CAPACITY: usize = 16;

/// Amiga scancodes for qualifier and control keys
pub const SCANCODE_CTRL: u8 = 0x63;
pub const SCANCODE_L_AMIGA: u8 = 0x66;
pub const SCANCODE_R_AMIGA: u8 = 0x67;
pub const SCANCODE_CAPS_LOCK: u8 = 0x62;

/// Out-of-band keyboard status and protocol codes
pub const SCANCODE_RESET_WARNING: u8 = 0x78;
pub const SCANCODE_LOST_SYNC: u8 = 0xF9;
pub const SCANCODE_BUFFER_OVERFLOW: u8 = 0xFA;
pub const SCANCODE_SELF_TEST_FAILED: u8 = 0xFC;
pub const SCANCODE_POWERUP_STREAM_START: u8 = 0xFD;
pub const SCANCODE_POWERUP_STREAM_END: u8 = 0xFE;

/// Keyboard serial transmission state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KeyboardTransmissionState {
    /// Idle, ready to transmit the next queued scancode
    #[default]
    Idle,
    /// Byte transmitted to CIA-A; awaiting software handshake (KDAT low pulse)
    WaitingHandshake,
}

/// Amiga keyboard controller and scancode encoder
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyboard {
    /// Active scancode holding register (currently presented to receiver)
    pub current_scancode: Option<u8>,
    /// Serial transmission state machine
    pub transmission_state: KeyboardTransmissionState,
    /// Fixed-capacity circular FIFO buffer for type-ahead keystrokes
    pub queue: [u8; KEYBOARD_BUFFER_CAPACITY],
    /// Number of scancodes currently stored in queue
    pub queue_len: usize,
    /// Read index of the FIFO buffer
    pub queue_head: usize,
    /// Control key active latch
    pub ctrl_pressed: bool,
    /// Left Amiga key active latch
    pub l_amiga_pressed: bool,
    /// Right Amiga key active latch
    pub r_amiga_pressed: bool,
    /// Caps Lock LED state (true = enabled)
    pub caps_lock_active: bool,
    /// True if the physical _RESET line is pulled low (Ctrl-Amiga-Amiga asserted)
    pub reset_line_asserted: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            current_scancode: None,
            transmission_state: KeyboardTransmissionState::Idle,
            queue: [0; KEYBOARD_BUFFER_CAPACITY],
            queue_len: 0,
            queue_head: 0,
            ctrl_pressed: false,
            l_amiga_pressed: false,
            r_amiga_pressed: false,
            caps_lock_active: false,
            reset_line_asserted: false,
        }
    }
}

impl Keyboard {
    /// Creates a new keyboard instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets keyboard state
    pub fn reset(&mut self) {
        self.current_scancode = None;
        self.transmission_state = KeyboardTransmissionState::Idle;
        self.queue = [0; KEYBOARD_BUFFER_CAPACITY];
        self.queue_len = 0;
        self.queue_head = 0;
        self.ctrl_pressed = false;
        self.l_amiga_pressed = false;
        self.r_amiga_pressed = false;
        self.caps_lock_active = false;
        self.reset_line_asserted = false;
    }

    /// Enqueues a pre-encoded scancode into the circular buffer.
    /// If full, reports buffer overflow by replacing newest entry with SCANCODE_BUFFER_OVERFLOW.
    pub fn enqueue_scancode(&mut self, encoded: u8) {
        if self.queue_len < KEYBOARD_BUFFER_CAPACITY {
            let tail = (self.queue_head + self.queue_len) % KEYBOARD_BUFFER_CAPACITY;
            self.queue[tail] = encoded;
            self.queue_len += 1;
        } else {
            let tail = (self.queue_head + self.queue_len - 1) % KEYBOARD_BUFFER_CAPACITY;
            self.queue[tail] = SCANCODE_BUFFER_OVERFLOW;
        }
    }

    /// Dequeues the next pending scancode from the circular buffer
    pub fn dequeue_scancode(&mut self) -> Option<u8> {
        if self.queue_len == 0 {
            None
        } else {
            let val = self.queue[self.queue_head];
            self.queue_head = (self.queue_head + 1) % KEYBOARD_BUFFER_CAPACITY;
            self.queue_len -= 1;
            Some(val)
        }
    }

    /// Returns true if there are scancodes pending in the type-ahead FIFO queue
    #[inline]
    pub fn has_pending_scancodes(&self) -> bool {
        self.queue_len > 0
    }

    /// Dispatches a key press event with raw Amiga scancode
    pub fn key_down(&mut self, raw_code: u8) {
        match raw_code {
            SCANCODE_CTRL => self.ctrl_pressed = true,
            SCANCODE_L_AMIGA => self.l_amiga_pressed = true,
            SCANCODE_R_AMIGA => self.r_amiga_pressed = true,
            SCANCODE_CAPS_LOCK => {
                self.caps_lock_active = !self.caps_lock_active;
                // Caps Lock transmits only on key-down:
                // When toggled on: transmits $62 (bit 7 = 0)
                // When toggled off: transmits $E2 (bit 7 = 1)
                let code = if self.caps_lock_active {
                    (SCANCODE_CAPS_LOCK << 1) & 0xFE
                } else {
                    ((SCANCODE_CAPS_LOCK << 1) & 0xFE) | 0x01
                };
                self.current_scancode = Some(code);
                self.enqueue_scancode(code);
                return;
            }
            _ => {}
        }

        if self.ctrl_pressed && self.l_amiga_pressed && self.r_amiga_pressed {
            self.reset_line_asserted = true;
            let warn = (SCANCODE_RESET_WARNING << 1) & 0xFE;
            self.enqueue_scancode(warn);
        }

        // Amiga scancode transmission format: (raw_code << 1) | 0
        let encoded = (raw_code << 1) & 0xFE;
        self.current_scancode = Some(encoded);
        self.enqueue_scancode(encoded);
    }

    /// Dispatches a key release event with raw Amiga scancode
    pub fn key_up(&mut self, raw_code: u8) {
        match raw_code {
            SCANCODE_CTRL => self.ctrl_pressed = false,
            SCANCODE_L_AMIGA => self.l_amiga_pressed = false,
            SCANCODE_R_AMIGA => self.r_amiga_pressed = false,
            SCANCODE_CAPS_LOCK => {
                // Caps Lock does not transmit on key release
                return;
            }
            _ => {}
        }

        // Key release bit is set: (raw_code << 1) | 1
        let encoded = ((raw_code << 1) & 0xFE) | 0x01;
        self.current_scancode = Some(encoded);
        self.enqueue_scancode(encoded);
    }

    /// Steps the keyboard serial transmission state machine.
    ///
    /// If `kdat_handshake` is true (Amiga OS pulled KDAT low via CIA-A CRA bit 6),
    /// acknowledges the previous byte and transitions to Idle.
    ///
    /// If in Idle state and there are queued scancodes, dequeues and returns the next
    /// byte ready to be shifted into CIA-A SDR.
    pub fn step(&mut self, kdat_handshake: bool) -> Option<u8> {
        if kdat_handshake {
            self.acknowledge();
        }

        if self.transmission_state == KeyboardTransmissionState::Idle {
            if let Some(scancode) = self.dequeue_scancode() {
                self.current_scancode = Some(scancode);
                self.transmission_state = KeyboardTransmissionState::WaitingHandshake;
                return Some(scancode);
            }
        }
        None
    }

    /// Acknowledges reception of the current scancode
    #[inline]
    pub fn acknowledge(&mut self) {
        self.current_scancode = None;
        self.transmission_state = KeyboardTransmissionState::Idle;
    }

    /// Returns true and clears the reset line if Ctrl-Amiga-Amiga was triggered
    #[inline]
    pub fn poll_reset(&mut self) -> bool {
        let was_asserted = self.reset_line_asserted;
        self.reset_line_asserted = false;
        was_asserted
    }

    /// Enqueues the standard Amiga power-up scancode stream reporting keys held down during boot
    pub fn queue_powerup_stream(&mut self, held_keys: &[u8]) {
        self.enqueue_scancode(SCANCODE_POWERUP_STREAM_START);
        for &k in held_keys {
            self.enqueue_scancode((k << 1) & 0xFE);
        }
        self.enqueue_scancode(SCANCODE_POWERUP_STREAM_END);
    }
}
