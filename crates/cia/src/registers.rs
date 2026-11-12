//! Named CIA register access and falling-edge capture.

use crate::{Cia, CCK_PER_ECLOCK, E_WRITE_CAPTURE_PHASE};
use config::{stage_mutation, MutationMode};

pub const BIT_CONTROL_START: u8 = 0x01;
pub const BIT_CONTROL_ONE_SHOT: u8 = 0x08;
const BIT_CONTROL_LOAD: u8 = 0x10;
const BIT_CRA_UNUSED: u8 = 0x80;
const MASK_CRA_WRITE_ONLY: u8 = BIT_CRA_UNUSED | BIT_CONTROL_LOAD;
pub const BIT_CRA_SERIAL_OUTPUT: u8 = 0x40;
pub const BIT_CRB_INPUT_MODE_SHIFT: u8 = 5;
pub const MASK_CRB_INPUT_MODE: u8 = 0x03;
pub const CRB_COUNT_ECLOCK: u8 = 0;
pub const CRB_COUNT_CNT: u8 = 1;
pub const CRB_COUNT_TA_UNDERFLOW: u8 = 2;
pub const CRB_COUNT_TA_UNDERFLOW_CNT_HIGH: u8 = 3;
const BIT_CRB_ALARM_WRITE: u8 = 0x80;

pub const BIT_ICR_TIMER_A: u8 = 0x01;
pub const BIT_ICR_TIMER_B: u8 = 0x02;
pub const BIT_ICR_ALARM: u8 = 0x04;
pub const BIT_ICR_FLAG: u8 = 0x10;
pub const MASK_ICR_SOURCE: u8 = 0x1F;
pub const BIT_ICR_PENDING: u8 = 0x80;
const BIT_ICR_SET: u8 = 0x80;

pub const MASK_TOD_COUNTER: u32 = 0x00FF_FFFF;
const MASK_TOD_LOW_WRITE_PRESERVE: u32 = 0xFFFF00;
const MASK_TOD_MIDDLE_WRITE_PRESERVE: u32 = 0xFF00FF;
const MASK_TOD_HIGH_WRITE_PRESERVE: u32 = 0x00FFFF;

const REGISTER_PRA: u16 = 0;
const REGISTER_PRB: u16 = 1;
const REGISTER_DDRA: u16 = 2;
const REGISTER_DDRB: u16 = 3;
const REGISTER_TALO: u16 = 4;
const REGISTER_TAHI: u16 = 5;
const REGISTER_TBLO: u16 = 6;
const REGISTER_TBHI: u16 = 7;
const REGISTER_TODLO: u16 = 8;
const REGISTER_TODMID: u16 = 9;
const REGISTER_TODHI: u16 = 10;
const REGISTER_SDR: u16 = 12;
const REGISTER_ICR: u16 = 13;
const REGISTER_CRA: u16 = 14;
const REGISTER_CRB: u16 = 15;

impl Cia {
    /// Returns the visible PRA value.
    #[inline]
    pub fn pra(&self) -> u8 {
        (self.pra & self.ddra) | (self.pins.port_a_input & !self.ddra)
    }

    /// Returns the visible PRB value.
    #[inline]
    pub fn prb(&self) -> u8 {
        (self.prb & self.ddrb) | (self.pins.port_b_input & !self.ddrb)
    }

    /// Returns the visible DDRA value.
    #[inline]
    pub fn ddra(&self) -> u8 {
        self.ddra
    }

    /// Returns the visible DDRB value.
    #[inline]
    pub fn ddrb(&self) -> u8 {
        self.ddrb
    }

    /// Returns the visible TALO value.
    #[inline]
    pub fn talo(&self) -> u8 {
        self.ta_counter as u8
    }

    /// Returns the visible TAHI value.
    #[inline]
    pub fn tahi(&self) -> u8 {
        (self.ta_counter >> 8) as u8
    }

    /// Returns the visible TBLO value.
    #[inline]
    pub fn tblo(&self) -> u8 {
        self.tb_counter as u8
    }

    /// Returns the visible TBHI value.
    #[inline]
    pub fn tbhi(&self) -> u8 {
        (self.tb_counter >> 8) as u8
    }

    /// Returns the visible SDR value.
    #[inline]
    pub fn sdr(&self) -> u8 {
        self.sdr
    }

    /// Returns the visible CRA value.
    #[inline]
    pub fn cra(&self) -> u8 {
        self.cra & !MASK_CRA_WRITE_ONLY
    }

    /// Returns the visible CRB value.
    #[inline]
    pub fn crb(&self) -> u8 {
        self.crb & !BIT_CONTROL_LOAD
    }

    /// Returns and acknowledges pending interrupt status at the accepted read phase.
    pub fn icr(&mut self) -> u8 {
        let value = self.icr_data;
        self.icr_data = 0;
        value
    }

    /// Inspects pending interrupts without acknowledging them.
    #[inline]
    pub fn icr_debug(&self) -> u8 {
        self.icr_data
    }

    /// Reads the held or live low byte and schedules release at the capture edge.
    pub fn todlo(&mut self) -> u8 {
        let value = self.visible_tod() as u8;
        self.tod_read_latch = Some(false);
        value
    }

    /// Returns the held or live middle byte without changing the latch.
    #[inline]
    pub fn todmid(&self) -> u8 {
        (self.visible_tod() >> 8) as u8
    }

    /// Reads the high byte and holds the counter on the falling edge.
    pub fn todhi(&mut self) -> u8 {
        let value = self.visible_tod();
        self.tod_read_value = value;
        self.tod_read_latch = Some(true);
        (value >> 16) as u8
    }

    #[inline]
    fn visible_tod(&self) -> u32 {
        if self.tod_latched {
            self.tod_latch_val
        } else {
            self.tod
        }
    }

    /// Inspects the live TOD low byte without releasing a latch.
    #[inline]
    pub fn todlo_debug(&self) -> u8 {
        self.tod as u8
    }
    /// Inspects the live TOD middle byte without modifying the latch.
    #[inline]
    pub fn todmid_debug(&self) -> u8 {
        (self.tod >> 8) as u8
    }
    /// Inspects the live TOD high byte without acquiring a latch.
    #[inline]
    pub fn todhi_debug(&self) -> u8 {
        (self.tod >> 16) as u8
    }

    /// Retains the last pending write per selector until the next falling E edge.
    /// Recomputing the remaining interval preserves the original capture boundary.
    fn schedule_register(&mut self, selector: u16, value: u8) -> bool {
        let delay = if self.eclock_phase < E_WRITE_CAPTURE_PHASE {
            E_WRITE_CAPTURE_PHASE - self.eclock_phase
        } else {
            CCK_PER_ECLOCK + E_WRITE_CAPTURE_PHASE - self.eclock_phase
        };
        stage_mutation(
            &mut self.mutations,
            selector,
            u16::from(value),
            delay,
            MutationMode::OverwritePending,
        )
    }

    /// Queues PRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_pra(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_PRA, value)
    }

    fn apply_pra(&mut self, val: u8) {
        self.prev_pra = self.pra;
        self.pra = val;
        self.pra_mutated = true;
    }

    /// Queues PRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_prb(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_PRB, value)
    }

    fn apply_prb(&mut self, val: u8) {
        self.prev_prb = self.prb;
        self.prb = val;
        self.prb_mutated = true;
    }

    /// Queues DDRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_ddra(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_DDRA, value)
    }

    fn apply_ddra(&mut self, val: u8) {
        self.ddra = val;
        self.pra_mutated = true;
    }

    /// Queues DDRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_ddrb(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_DDRB, value)
    }

    fn apply_ddrb(&mut self, val: u8) {
        self.ddrb = val;
        self.prb_mutated = true;
    }

    /// Queues TALO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_talo(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TALO, value)
    }

    fn apply_talo(&mut self, val: u8) {
        self.ta_latch = (self.ta_latch & 0xFF00) | (val as u16)
    }

    /// Queues TAHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tahi(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TAHI, value)
    }

    fn apply_tahi(&mut self, val: u8) {
        self.ta_latch = (self.ta_latch & 0x00FF) | ((val as u16) << 8);
        if (self.cra & BIT_CONTROL_START) == 0 || (self.cra & BIT_CONTROL_ONE_SHOT) != 0 {
            self.ta_counter = self.ta_latch;
        }
        if (self.cra & BIT_CONTROL_ONE_SHOT) != 0 {
            self.cra |= BIT_CONTROL_START;
        }
    }

    /// Queues TBLO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tblo(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TBLO, value)
    }

    fn apply_tblo(&mut self, val: u8) {
        self.tb_latch = (self.tb_latch & 0xFF00) | (val as u16)
    }

    /// Queues TBHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tbhi(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TBHI, value)
    }

    fn apply_tbhi(&mut self, val: u8) {
        self.tb_latch = (self.tb_latch & 0x00FF) | ((val as u16) << 8);
        if (self.crb & BIT_CONTROL_START) == 0 || (self.crb & BIT_CONTROL_ONE_SHOT) != 0 {
            self.tb_counter = self.tb_latch;
        }
        if (self.crb & BIT_CONTROL_ONE_SHOT) != 0 {
            self.crb |= BIT_CONTROL_START;
        }
    }

    /// Queues TODLO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todlo(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TODLO, value)
    }

    fn apply_todlo(&mut self, val: u8) {
        if (self.crb & BIT_CRB_ALARM_WRITE) != 0 {
            self.tod_alarm = (self.tod_alarm & MASK_TOD_LOW_WRITE_PRESERVE) | (val as u32);
        } else {
            self.tod = (self.tod & MASK_TOD_LOW_WRITE_PRESERVE) | (val as u32);
            self.tod_halted = false; // Writing TODLO restarts counter
        }
    }

    /// Queues TODMID; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todmid(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TODMID, value)
    }

    fn apply_todmid(&mut self, val: u8) {
        if (self.crb & BIT_CRB_ALARM_WRITE) != 0 {
            self.tod_alarm =
                (self.tod_alarm & MASK_TOD_MIDDLE_WRITE_PRESERVE) | ((val as u32) << 8);
        } else {
            self.tod = (self.tod & MASK_TOD_MIDDLE_WRITE_PRESERVE) | ((val as u32) << 8);
            self.tod_halted = true;
        }
    }

    /// Queues TODHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todhi(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_TODHI, value)
    }

    fn apply_todhi(&mut self, val: u8) {
        if (self.crb & BIT_CRB_ALARM_WRITE) != 0 {
            self.tod_alarm = (self.tod_alarm & MASK_TOD_HIGH_WRITE_PRESERVE) | ((val as u32) << 16);
        } else {
            self.tod = (self.tod & MASK_TOD_HIGH_WRITE_PRESERVE) | ((val as u32) << 16);
            self.tod_halted = true; // Writing TODHI halts counter until TODLO is written
        }
    }

    /// Queues SDR; returns false when capacity is exhausted.
    #[inline]
    pub fn set_sdr(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_SDR, value)
    }

    fn apply_sdr(&mut self, val: u8) {
        self.sdr = val
    }

    /// Queues ICR; returns false when capacity is exhausted.
    #[inline]
    pub fn set_icr(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_ICR, value)
    }

    fn apply_icr(&mut self, val: u8) {
        // SET/CLR bit 7 logic for ICR mask
        if (val & BIT_ICR_SET) != 0 {
            self.icr_mask |= val & MASK_ICR_SOURCE;
        } else {
            self.icr_mask &= !(val & MASK_ICR_SOURCE);
        }
        if self.icr_data & self.icr_mask & MASK_ICR_SOURCE != 0 {
            self.icr_data |= BIT_ICR_PENDING;
        }
    }

    /// Queues CRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_cra(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_CRA, value)
    }

    fn apply_cra(&mut self, val: u8) {
        self.cra = val & !MASK_CRA_WRITE_ONLY;
        if (val & BIT_CONTROL_LOAD) != 0 {
            self.ta_counter = self.ta_latch; // Force load
        }
    }

    /// Queues CRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_crb(&mut self, value: u8) -> bool {
        self.schedule_register(REGISTER_CRB, value)
    }

    fn apply_crb(&mut self, val: u8) {
        self.crb = val & !BIT_CONTROL_LOAD;
        if (val & BIT_CONTROL_LOAD) != 0 {
            self.tb_counter = self.tb_latch; // Force load
        }
    }

    pub(super) fn apply_register(&mut self, selector: u16, value: u8) {
        match selector {
            REGISTER_PRA => self.apply_pra(value),
            REGISTER_PRB => self.apply_prb(value),
            REGISTER_DDRA => self.apply_ddra(value),
            REGISTER_DDRB => self.apply_ddrb(value),
            REGISTER_TALO => self.apply_talo(value),
            REGISTER_TAHI => self.apply_tahi(value),
            REGISTER_TBLO => self.apply_tblo(value),
            REGISTER_TBHI => self.apply_tbhi(value),
            REGISTER_TODLO => self.apply_todlo(value),
            REGISTER_TODMID => self.apply_todmid(value),
            REGISTER_TODHI => self.apply_todhi(value),
            REGISTER_SDR => self.apply_sdr(value),
            REGISTER_ICR => self.apply_icr(value),
            REGISTER_CRA => self.apply_cra(value),
            REGISTER_CRB => self.apply_crb(value),
            _ => {}
        }
    }
}
