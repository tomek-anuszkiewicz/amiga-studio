//! MOS 8520 Complex Interface Adapter (CIA-A & CIA-B) Emulation
//!
//! Reusable 8520 chip core with 16-bit decrementing Timers A & B,
//! bidirectional 8-bit Ports A & B, 24-bit TOD clock, SDR, and ICR.

mod registers;

use config::{tick_mutations, DelayedMutation};
use serde::{Deserialize, Serialize};

/// Number of Color Clocks per Motorola E-Clock tick (5 CCK = 10 CPU cycles)
const CCK_PER_ECLOCK: u8 = 5;
/// Falling E edge in the current CCK phase convention.
const E_WRITE_CAPTURE_PHASE: u8 = 2;
/// CPU transfer acceptance occurs one CCK before falling E.
const E_BUS_ACCESS_PHASE: u8 = E_WRITE_CAPTURE_PHASE - 1;
/// Minimum attempt-to-acceptance interval reproducing vAmiga's even-phase table.
/// This is a model setting in CCK, not a measured pin propagation delay.
const MIN_BUS_SYNC_CCK: u8 = 3;

/// Total number of 8520 registers
const CIA_REGISTER_COUNT: usize = 16;

/// MOS 8520 CIA chip state
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Cia {
    /// Port A Data Register (PRA)
    pub pra: u8,
    /// Port B Data Register (PRB)
    pub prb: u8,
    /// Previously latched Port A data (for edge / pin transition detection)
    pub prev_pra: u8,
    /// Previously latched Port B data (for edge / pin transition detection)
    pub prev_prb: u8,
    /// True if Port A was written and output pins mutated
    pub pra_mutated: bool,
    /// True if Port B was written and output pins mutated
    pub prb_mutated: bool,
    /// Data Direction Register A (DDRA: 1 = output, 0 = input)
    pub ddra: u8,
    /// Data Direction Register B (DDRB: 1 = output, 0 = input)
    pub ddrb: u8,
    /// Timer A reload latch (16-bit)
    pub ta_latch: u16,
    /// Timer A current down-counter (16-bit)
    pub ta_counter: u16,
    /// Control Register A (CRA)
    pub cra: u8,
    /// Timer B reload latch (16-bit)
    pub tb_latch: u16,
    /// Timer B current down-counter (16-bit)
    pub tb_counter: u16,
    /// Control Register B (CRB)
    pub crb: u8,
    /// 24-bit Time-of-Day counter
    pub tod: u32,
    /// 24-bit Time-of-Day alarm register
    pub tod_alarm: u32,
    /// True if TOD read latch is currently frozen
    pub tod_latched: bool,
    /// Frozen TOD snapshot value captured upon reading TODHI
    pub tod_latch_val: u32,
    /// True if TOD counting is halted by a middle/high time write until TODLO is written
    pub tod_halted: bool,
    /// Serial Data Register (shift register)
    pub sdr: u8,
    /// Interrupt Control Register data/requests (read: cleared on read)
    pub icr_data: u8,
    /// Interrupt Control Register mask/enables (write)
    pub icr_mask: u8,
    /// Internal Color Clock divider phase to E-Clock (0..4)
    pub eclock_phase: u8,

    /// Remaining CCKs in a synchronized CPU bus access; zero awaits acknowledgment.
    pub bus_wait_cck: Option<u8>,
    /// Pending TOD latch acquire/release, applied at the falling E edge.
    pub tod_read_latch: Option<bool>,
    /// Captured value for a pending TOD high-byte read.
    pub tod_read_value: u32,

    /// Fixed inline in-flight register mutation buffer for CIA
    pub mutations: [Option<DelayedMutation>; CIA_REGISTER_COUNT],
}

impl Cia {
    /// Creates a new MOS 8520 instance
    pub fn new() -> Self {
        Self {
            ta_latch: 0xFFFF,
            ta_counter: 0xFFFF,
            tb_latch: 0xFFFF,
            tb_counter: 0xFFFF,
            ..Default::default()
        }
    }

    /// Resets all CIA registers to power-on defaults
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Advances the E-clock phase: rising at 0, falling at 2 (two CCK high).
    pub fn step_cck(&mut self) {
        self.eclock_phase = self.eclock_phase.wrapping_add(1) % CCK_PER_ECLOCK;
        self.bus_wait_cck = self.bus_wait_cck.map(|n| n.saturating_sub(1));
        let mut due = [None; CIA_REGISTER_COUNT];
        let mut due_count = 0;
        tick_mutations(&mut self.mutations, |selector, value| {
            due[due_count] = Some((selector, value as u8));
            due_count += 1;
        });
        for (selector, value) in due.into_iter().flatten() {
            self.apply_register(selector, value);
        }
        if self.eclock_phase == E_WRITE_CAPTURE_PHASE {
            if let Some(latched) = self.tod_read_latch.take() {
                self.tod_latched = latched;
                if latched {
                    self.tod_latch_val = self.tod_read_value;
                }
            }
        }
        if self.eclock_phase == 0 {
            self.step_eclock();
        }
    }

    /// Prepares a stable CPU access for the penultimate high E-clock CCK.
    /// Readiness is retained until the motherboard accepts the whole transfer.
    /// The five delays are the even CPU-phase entries in vAmiga's E-clock table.
    pub fn synchronize_bus_access(&mut self) -> bool {
        match self.bus_wait_cck {
            None => {
                let cck_until_access =
                    (CCK_PER_ECLOCK + E_BUS_ACCESS_PHASE - self.eclock_phase) % CCK_PER_ECLOCK;
                // If the nearest acceptance window is too close, use the next E period.
                let wait = if cck_until_access < MIN_BUS_SYNC_CCK {
                    cck_until_access + CCK_PER_ECLOCK
                } else {
                    cck_until_access
                };
                self.bus_wait_cck = Some(wait);
                false
            }
            Some(0) if self.eclock_phase == E_BUS_ACCESS_PHASE => true,
            Some(_) => false,
        }
    }

    /// Completes synchronization after every receiving chip is ready.
    #[inline]
    pub fn finish_bus_access(&mut self) {
        self.bus_wait_cck = None;
    }

    /// Ticks the existing timer engines on the rising E edge.
    fn step_eclock(&mut self) {
        // 2. Timer A down-count (if CRA bit 0 is set)
        let mut ta_underflow = false;
        if (self.cra & 0x01) != 0 {
            if self.ta_counter == 0 {
                self.ta_counter = self.ta_latch;
                self.trigger_icr(0x01); // Bit 0: Timer A underflow
                ta_underflow = true;
                if (self.cra & 0x08) != 0 {
                    self.cra &= !0x01; // One-shot mode: stop timer
                }
            } else {
                self.ta_counter = self.ta_counter.wrapping_sub(1);
            }
        }

        // 3. Timer B down-count (if CRB bit 0 is set)
        if (self.crb & 0x01) != 0 {
            let inmode = (self.crb >> 5) & 0x03;
            let tb_decrement = match inmode {
                0x00 => true,                // Counts E-Clock pulses
                0x02 | 0x03 => ta_underflow, // Cascaded 32-bit counting: Timer A underflow pulses
                _ => false,                  // 0x01: Counts CNT pin pulses (handled via step_cnt)
            };

            if tb_decrement {
                if self.tb_counter == 0 {
                    self.tb_counter = self.tb_latch;
                    self.trigger_icr(0x02); // Bit 1: Timer B underflow
                    if (self.crb & 0x08) != 0 {
                        self.crb &= !0x01; // One-shot mode: stop timer
                    }
                } else {
                    self.tb_counter = self.tb_counter.wrapping_sub(1);
                }
            }
        }
    }

    /// Sets ICR request bit and updates master IRQ flag
    pub fn trigger_icr(&mut self, bit: u8) {
        self.icr_data |= bit & 0x1F;
        if (self.icr_data & self.icr_mask & 0x1F) != 0 {
            self.icr_data |= 0x80; // Master interrupt pending bit
        }
    }

    /// Returns true if an unmasked interrupt request is currently pending
    #[inline]
    pub fn irq_pending(&self) -> bool {
        (self.icr_data & 0x80) != 0
    }

    /// Injects external peripheral input pin levels into Port A for bits configured as inputs (`!ddra`)
    #[inline]
    pub fn set_input_pins_a(&mut self, pins: u8, mask: u8) {
        let input_mask = mask & !self.ddra;
        self.pra = (self.pra & !input_mask) | (pins & input_mask);
    }

    /// Polls whether Port A output data was written, clearing the flag and returning the written byte
    #[inline]
    pub fn poll_pra_output(&mut self) -> Option<u8> {
        if self.pra_mutated {
            self.pra_mutated = false;
            Some(self.pra)
        } else {
            None
        }
    }

    /// Polls a Port B latch/direction change and returns its effective pulled-up pin levels
    #[inline]
    pub fn poll_prb_output(&mut self) -> Option<u8> {
        if self.prb_mutated {
            self.prb_mutated = false;
            Some(self.prb | !self.ddrb)
        } else {
            None
        }
    }

    /// Advances the 24-bit Time-of-Day (TOD) counter by 1 tick.
    /// In CIA-A, this is connected to 50/60 Hz VSync ticks.
    /// In CIA-B, this is connected to horizontal scanline HSync ticks.
    ///
    /// If counter increment is not halted and matches the alarm register,
    /// asserts the TOD alarm interrupt in ICR (bit 2).
    pub fn tick_tod(&mut self) {
        if !self.tod_halted {
            self.tod = (self.tod.wrapping_add(1)) & 0x00FF_FFFF;
            if self.tod == (self.tod_alarm & 0x00FF_FFFF) {
                self.trigger_icr(0x04); // Bit 2: TOD alarm match
            }
        }
    }

    /// Clocks an external transition on the CNT pin.
    /// If Timer B is in CNT count mode (CRB bits 6..5 == %01), decrements Timer B.
    pub fn step_cnt(&mut self) {
        if (self.crb & 0x01) != 0 && ((self.crb >> 5) & 0x03) == 0x01 {
            if self.tb_counter == 0 {
                self.tb_counter = self.tb_latch;
                self.trigger_icr(0x02); // Bit 1: Timer B underflow
                if (self.crb & 0x08) != 0 {
                    self.crb &= !0x01; // One-shot mode: stop timer
                }
            } else {
                self.tb_counter = self.tb_counter.wrapping_sub(1);
            }
        }
    }

    /// Shifts an 8-bit byte serially into the SDR from an external device (e.g. keyboard),
    /// latching the byte and asserting the SDR interrupt in ICR (bit 3).
    pub fn shift_in_sdr(&mut self, val: u8) {
        self.sdr = val;
        self.trigger_icr(0x08); // Bit 3: SDR shift complete
    }

    /// Returns true if the SDR is configured in serial output mode (CRA bit 6 = 1)
    #[inline]
    pub fn is_sdr_output(&self) -> bool {
        (self.cra & 0x40) != 0
    }

    /// Triggers the FLAG pin negative edge interrupt (ICR bit 4).
    /// Connected to parallel port ACK on CIA-A, serial port CD on CIA-B.
    #[inline]
    pub fn trigger_flag_pin(&mut self) {
        self.trigger_icr(0x10); // Bit 4: FLAG pin transition
    }
}
