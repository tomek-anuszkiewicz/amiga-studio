//! MOS 8520 Complex Interface Adapter (CIA-A & CIA-B) Emulation
//!
//! Reusable 8520 chip core with 16-bit decrementing Timers A & B,
//! bidirectional 8-bit Ports A & B, 24-bit TOD clock, SDR, and ICR.

mod pins;
mod ports;
mod registers;
mod serial;
mod timers;
pub use pins::{CiaInputs, CiaPins};
use registers::{
    BIT_CRA_SERIAL_OUTPUT, BIT_ICR_ALARM, BIT_ICR_FLAG, BIT_ICR_PENDING, MASK_ICR_SOURCE,
    MASK_TOD_COUNTER,
};
use serial::SerialState;

use config::MutationQueue;
use serde::{Deserialize, Serialize};

/// Number of Color Clocks per Motorola E-Clock tick (5 CCK = 10 CPU cycles)
const CCK_PER_ECLOCK: u8 = 5;
/// Falling E edge in the current CCK phase convention.
const E_WRITE_CAPTURE_PHASE: u8 = 2;
const E_TIMER_TICK_PHASE: u8 = 0;
const TIMER_RESET_VALUE: u16 = 0xFFFF;
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
    /// Input capture and output drive boundary for motherboard serial/handshake lines.
    pub pins: CiaPins,
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

    /// Captured positive CNT transition, consumed by the engine once.
    cnt_rising: bool,
    serial: SerialState,
    timer_a_output: bool,
    timer_b_output: bool,
    timer_a_pulse: u8,
    timer_b_pulse: u8,
    /// Port B access pulses scheduled for the third following E cycle.
    pc_pipeline: u8,

    /// Fixed inline in-flight register mutation buffer for CIA
    pub mutations: MutationQueue<{ CIA_REGISTER_COUNT }>,
}

impl Cia {
    /// Creates a new MOS 8520 instance
    pub fn new() -> Self {
        Self {
            ta_latch: TIMER_RESET_VALUE,
            ta_counter: TIMER_RESET_VALUE,
            tb_latch: TIMER_RESET_VALUE,
            tb_counter: TIMER_RESET_VALUE,
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
        self.mutations.tick();
        for index in 0..self.mutations.due().len() {
            let (selector, value) = self.mutations.due()[index];
            self.apply_register(selector, value as u8);
        }
        if self.eclock_phase == E_WRITE_CAPTURE_PHASE {
            if let Some(latched) = self.tod_read_latch.take() {
                self.tod_latched = latched;
                if latched {
                    self.tod_latch_val = self.tod_read_value;
                }
            }
        }
        let e_rising = self.eclock_phase == E_TIMER_TICK_PHASE;
        let cnt_rising = core::mem::take(&mut self.cnt_rising);
        self.step_timers(e_rising, cnt_rising);
        if cnt_rising && !self.is_sdr_output() {
            self.shift_serial_input();
        }
        if e_rising {
            self.step_port_handshake();
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

    /// Sets ICR request bit and updates master IRQ flag
    pub fn trigger_icr(&mut self, bit: u8) {
        self.icr_data |= bit & MASK_ICR_SOURCE;
        if (self.icr_data & self.icr_mask & MASK_ICR_SOURCE) != 0 {
            self.icr_data |= BIT_ICR_PENDING; // Master interrupt pending bit
        }
    }

    /// Returns true if an unmasked interrupt request is currently pending
    #[inline]
    pub fn irq_pending(&self) -> bool {
        (self.icr_data & BIT_ICR_PENDING) != 0
    }

    /// Advances the 24-bit Time-of-Day (TOD) counter by 1 tick.
    /// In CIA-A, this is connected to 50/60 Hz VSync ticks.
    /// In CIA-B, this is connected to horizontal scanline HSync ticks.
    ///
    /// If counter increment is not halted and matches the alarm register,
    /// asserts the TOD alarm interrupt in ICR (bit 2).
    pub fn tick_tod(&mut self) {
        if !self.tod_halted {
            self.tod = (self.tod.wrapping_add(1)) & MASK_TOD_COUNTER;
            if self.tod == (self.tod_alarm & MASK_TOD_COUNTER) {
                self.trigger_icr(BIT_ICR_ALARM); // Bit 2: TOD alarm match
            }
        }
    }

    /// Returns true if the SDR is configured in serial output mode (CRA bit 6 = 1)
    #[inline]
    pub fn is_sdr_output(&self) -> bool {
        (self.cra & BIT_CRA_SERIAL_OUTPUT) != 0
    }

    /// Triggers the FLAG pin negative edge interrupt (ICR bit 4).
    /// Connected to parallel port ACK on CIA-A and disk INDEX on CIA-B.
    #[inline]
    pub fn trigger_flag_pin(&mut self) {
        self.trigger_icr(BIT_ICR_FLAG); // Bit 4: FLAG pin transition
    }
}
