//! MOS 8520 Complex Interface Adapter (CIA-A & CIA-B) Emulation
//!
//! Reusable 8520 chip core with 16-bit decrementing Timers A & B,
//! bidirectional 8-bit Ports A & B, 24-bit TOD clock, SDR, and ICR.

use config::{stage_mutation, tick_mutations, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Number of Color Clocks per Motorola E-Clock tick (5 CCK = 10 CPU cycles)
pub const CCK_PER_ECLOCK: u8 = 5;

/// Total number of 8520 registers
pub const CIA_REGISTER_COUNT: usize = 16;

/// CIA chip identity (CIA-A or CIA-B)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiaId {
    /// CIA-A (connected to IRQ level 2, keyboard, game port fire, OVL)
    A,
    /// CIA-B (connected to IRQ level 6, parallel port, floppy control)
    B,
}

impl Default for CiaId {
    fn default() -> Self {
        CiaId::A
    }
}

/// MOS 8520 CIA chip state
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Cia {
    /// CIA chip identifier
    pub id: CiaId,
    /// Port A Data Register (PRA)
    pub pra: u8,
    /// Port B Data Register (PRB)
    pub prb: u8,
    /// Previously latched Port A data (for edge / pin transition detection)
    pub prev_pra: u8,
    /// Previously latched Port B data (for edge / pin transition detection)
    pub prev_prb: u8,
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
    /// Serial Data Register (shift register)
    pub sdr: u8,
    /// Interrupt Control Register data/requests (read: cleared on read)
    pub icr_data: u8,
    /// Interrupt Control Register mask/enables (write)
    pub icr_mask: u8,
    /// Internal Color Clock divider phase to E-Clock (0..4)
    pub eclock_phase: u8,

    /// Fixed inline in-flight register mutation buffer for CIA
    pub mutations: [Option<DelayedMutation>; CIA_REGISTER_COUNT],
}

impl Cia {
    /// Creates a new CIA instance for the given identifier
    pub fn new(id: CiaId) -> Self {
        Self {
            id,
            ta_latch: 0xFFFF,
            ta_counter: 0xFFFF,
            tb_latch: 0xFFFF,
            tb_counter: 0xFFFF,
            ..Default::default()
        }
    }

    /// Resets all CIA registers to power-on defaults
    pub fn reset(&mut self) {
        self.pra = 0;
        self.prb = 0;
        self.prev_pra = 0;
        self.prev_prb = 0;
        self.ddra = 0;
        self.ddrb = 0;
        self.ta_latch = 0xFFFF;
        self.ta_counter = 0xFFFF;
        self.cra = 0;
        self.tb_latch = 0xFFFF;
        self.tb_counter = 0xFFFF;
        self.crb = 0;
        self.tod = 0;
        self.tod_alarm = 0;
        self.tod_latched = false;
        self.tod_latch_val = 0;
        self.sdr = 0;
        self.icr_data = 0;
        self.icr_mask = 0;
        self.eclock_phase = 0;
        self.mutations = [None; CIA_REGISTER_COUNT];
    }

    /// Advances CIA internal timers and E-Clock prescaler by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self) -> [Option<(u8, u8)>; 4] {
        self.eclock_phase = self.eclock_phase.wrapping_add(1);
        if self.eclock_phase >= CCK_PER_ECLOCK {
            self.eclock_phase = 0;
            self.step_eclock()
        } else {
            [None; 4]
        }
    }

    /// Ticks CIA timers on each E-Clock pulse (every 5 CCKs)
    fn step_eclock(&mut self) -> [Option<(u8, u8)>; 4] {
        // 1. Process in-flight mutations scheduled for this E-Clock
        let mut due = [None; 4];
        let mut due_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if due_count < due.len() {
                due[due_count] = Some((reg as u8, val as u8));
                due_count += 1;
            }
        });
        for item in due.iter().flatten() {
            self.commit_register_write(item.0, item.1);
        }

        // 2. Timer A down-count (if CRA bit 0 is set)
        if (self.cra & 0x01) != 0 {
            if self.ta_counter == 0 {
                self.ta_counter = self.ta_latch;
                self.trigger_icr(0x01); // Bit 0: Timer A underflow
                if (self.cra & 0x08) != 0 {
                    self.cra &= !0x01; // One-shot mode: stop timer
                }
            } else {
                self.ta_counter = self.ta_counter.wrapping_sub(1);
            }
        }

        // 3. Timer B down-count (if CRB bit 0 is set)
        if (self.crb & 0x01) != 0 {
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

        due
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

    /// Reads CIA register ($0..$F) with clear-on-read for ICR and atomic latching for TOD
    pub fn read_register(&mut self, reg: u8) -> u8 {
        match reg & 0x0F {
            0x0 => self.pra,
            0x1 => self.prb,
            0x2 => self.ddra,
            0x3 => self.ddrb,
            0x4 => (self.ta_counter & 0xFF) as u8,
            0x5 => (self.ta_counter >> 8) as u8,
            0x6 => (self.tb_counter & 0xFF) as u8,
            0x7 => (self.tb_counter >> 8) as u8,
            0x8 => {
                // Reading TODLO unlatches TOD
                let val = if self.tod_latched {
                    (self.tod_latch_val & 0xFF) as u8
                } else {
                    (self.tod & 0xFF) as u8
                };
                self.tod_latched = false;
                val
            }
            0x9 => {
                if self.tod_latched {
                    ((self.tod_latch_val >> 8) & 0xFF) as u8
                } else {
                    ((self.tod >> 8) & 0xFF) as u8
                }
            }
            0xA => {
                // Reading TODHI freezes/latches TOD
                self.tod_latched = true;
                self.tod_latch_val = self.tod;
                ((self.tod >> 16) & 0xFF) as u8
            }
            0xC => self.sdr,
            0xD => {
                let val = self.icr_data;
                self.icr_data = 0; // Clear on read
                val
            }
            0xE => self.cra,
            0xF => self.crb,
            _ => 0xFF,
        }
    }

    /// Read-only inspection of register without side-effects (for debuggers)
    #[inline]
    pub fn peek_register(&self, reg: u8) -> u8 {
        match reg & 0x0F {
            0x0 => self.pra,
            0x1 => self.prb,
            0x2 => self.ddra,
            0x3 => self.ddrb,
            0x4 => (self.ta_counter & 0xFF) as u8,
            0x5 => (self.ta_counter >> 8) as u8,
            0x6 => (self.tb_counter & 0xFF) as u8,
            0x7 => (self.tb_counter >> 8) as u8,
            0x8 => (self.tod & 0xFF) as u8,
            0x9 => ((self.tod >> 8) & 0xFF) as u8,
            0xA => ((self.tod >> 16) & 0xFF) as u8,
            0xC => self.sdr,
            0xD => self.icr_data,
            0xE => self.cra,
            0xF => self.crb,
            _ => 0xFF,
        }
    }

    /// Stages a register write with E-Clock delay in the mutation pipeline.
    /// Returns `Some((reg, val))` if committed immediately, or `None` if staged in pipeline.
    pub fn stage_write(
        &mut self,
        reg: u8,
        val: u8,
        delay_eclocks: u8,
        mode: MutationMode,
    ) -> Option<(u8, u8)> {
        let reg = reg & 0x0F;
        if !stage_mutation(
            &mut self.mutations,
            reg as u16,
            val as u16,
            delay_eclocks,
            mode,
        ) {
            self.commit_register_write(reg, val);
            Some((reg, val))
        } else {
            None
        }
    }

    /// Writes CIA register directly upon completion of synchronous E-Clock bus cycle.
    /// Returns `Some((reg, val))` representing the committed write.
    pub fn write_register(&mut self, reg: u8, val: u8) -> Option<(u8, u8)> {
        let reg = reg & 0x0F;
        self.commit_register_write(reg, val);
        Some((reg, val))
    }

    /// Injects external peripheral input pin levels into Port A for bits configured as inputs (`!ddra`)
    #[inline]
    pub fn set_input_pins_a(&mut self, pins: u8, mask: u8) {
        let input_mask = mask & !self.ddra;
        self.pra = (self.pra & !input_mask) | (pins & input_mask);
    }

    /// Commits a register write directly into active CIA silicon state
    pub fn commit_register_write(&mut self, reg: u8, val: u8) {
        match reg & 0x0F {
            0x0 => {
                self.prev_pra = self.pra;
                self.pra = val;
            }
            0x1 => {
                self.prev_prb = self.prb;
                self.prb = val;
            }
            0x2 => self.ddra = val,
            0x3 => self.ddrb = val,
            0x4 => self.ta_latch = (self.ta_latch & 0xFF00) | (val as u16),
            0x5 => {
                self.ta_latch = (self.ta_latch & 0x00FF) | ((val as u16) << 8);
                if (self.cra & 0x01) == 0 {
                    self.ta_counter = self.ta_latch;
                }
            }
            0x6 => self.tb_latch = (self.tb_latch & 0xFF00) | (val as u16),
            0x7 => {
                self.tb_latch = (self.tb_latch & 0x00FF) | ((val as u16) << 8);
                if (self.crb & 0x01) == 0 {
                    self.tb_counter = self.tb_latch;
                }
            }
            0x8 => {
                // TOD write (low byte starts counter)
                self.tod = (self.tod & 0xFFFF00) | (val as u32);
            }
            0x9 => {
                self.tod = (self.tod & 0xFF00FF) | ((val as u32) << 8);
            }
            0xA => {
                // TOD write high byte halts counter until TODLO is written
                self.tod = (self.tod & 0x00FFFF) | ((val as u32) << 16);
            }
            0xC => self.sdr = val,
            0xD => {
                // SET/CLR bit 7 logic for ICR mask
                if (val & 0x80) != 0 {
                    self.icr_mask |= val & 0x1F;
                } else {
                    self.icr_mask &= !(val & 0x1F);
                }
            }
            0xE => {
                self.cra = val;
                if (val & 0x10) != 0 {
                    self.ta_counter = self.ta_latch; // Force load
                }
            }
            0xF => {
                self.crb = val;
                if (val & 0x10) != 0 {
                    self.tb_counter = self.tb_latch; // Force load
                }
            }
            _ => {}
        }
    }

    /// Checks if Port A bit 0 (`_OVL`) has transitioned
    #[inline]
    pub fn ovl_transition(&self) -> Option<bool> {
        let old = (self.prev_pra & 0x01) != 0;
        let new = (self.pra & 0x01) != 0;
        if old != new {
            Some(new)
        } else {
            None
        }
    }

    /// Checks if Port A bit 1 (`_LED`) has transitioned
    #[inline]
    pub fn led_transition(&self) -> Option<bool> {
        let old = (self.prev_pra & 0x02) != 0;
        let new = (self.pra & 0x02) != 0;
        if old != new {
            Some(new)
        } else {
            None
        }
    }
}
