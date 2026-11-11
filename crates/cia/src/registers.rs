//! Named CIA register access and falling-edge capture.

use crate::{Cia, CCK_PER_ECLOCK, E_WRITE_CAPTURE_PHASE};
use config::{stage_mutation, MutationMode};

impl Cia {
    /// Returns the visible PRA value.
    #[inline]
    pub fn pra(&self) -> u8 {
        self.pra
    }

    /// Returns the visible PRB value.
    #[inline]
    pub fn prb(&self) -> u8 {
        self.prb
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
        self.cra & !0x90
    }

    /// Returns the visible CRB value.
    #[inline]
    pub fn crb(&self) -> u8 {
        self.crb & !0x10
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

    /// Queues a write for the next falling E edge. Full queues reject the write.
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
            MutationMode::Pipeline,
        )
    }

    /// Queues PRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_pra(&mut self, value: u8) -> bool {
        self.schedule_register(0, value)
    }

    fn apply_pra(&mut self, val: u8) {
        self.prev_pra = self.pra;
        self.pra = val;
        self.pra_mutated = true;
    }

    /// Queues PRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_prb(&mut self, value: u8) -> bool {
        self.schedule_register(1, value)
    }

    fn apply_prb(&mut self, val: u8) {
        self.prev_prb = self.prb;
        self.prb = val;
        self.prb_mutated = true;
    }

    /// Queues DDRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_ddra(&mut self, value: u8) -> bool {
        self.schedule_register(2, value)
    }

    fn apply_ddra(&mut self, val: u8) {
        self.ddra = val;
        self.pra_mutated = true;
    }

    /// Queues DDRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_ddrb(&mut self, value: u8) -> bool {
        self.schedule_register(3, value)
    }

    fn apply_ddrb(&mut self, val: u8) {
        self.ddrb = val;
        self.prb_mutated = true;
    }

    /// Queues TALO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_talo(&mut self, value: u8) -> bool {
        self.schedule_register(4, value)
    }

    fn apply_talo(&mut self, val: u8) {
        self.ta_latch = (self.ta_latch & 0xFF00) | (val as u16)
    }

    /// Queues TAHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tahi(&mut self, value: u8) -> bool {
        self.schedule_register(5, value)
    }

    fn apply_tahi(&mut self, val: u8) {
        self.ta_latch = (self.ta_latch & 0x00FF) | ((val as u16) << 8);
        if (self.cra & 0x01) == 0 || (self.cra & 0x08) != 0 {
            self.ta_counter = self.ta_latch;
        }
        if (self.cra & 0x08) != 0 {
            self.cra |= 0x01;
        }
    }

    /// Queues TBLO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tblo(&mut self, value: u8) -> bool {
        self.schedule_register(6, value)
    }

    fn apply_tblo(&mut self, val: u8) {
        self.tb_latch = (self.tb_latch & 0xFF00) | (val as u16)
    }

    /// Queues TBHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_tbhi(&mut self, value: u8) -> bool {
        self.schedule_register(7, value)
    }

    fn apply_tbhi(&mut self, val: u8) {
        self.tb_latch = (self.tb_latch & 0x00FF) | ((val as u16) << 8);
        if (self.crb & 0x01) == 0 || (self.crb & 0x08) != 0 {
            self.tb_counter = self.tb_latch;
        }
        if (self.crb & 0x08) != 0 {
            self.crb |= 0x01;
        }
    }

    /// Queues TODLO; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todlo(&mut self, value: u8) -> bool {
        self.schedule_register(8, value)
    }

    fn apply_todlo(&mut self, val: u8) {
        if (self.crb & 0x80) != 0 {
            self.tod_alarm = (self.tod_alarm & 0xFFFF00) | (val as u32);
        } else {
            self.tod = (self.tod & 0xFFFF00) | (val as u32);
            self.tod_halted = false; // Writing TODLO restarts counter
        }
    }

    /// Queues TODMID; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todmid(&mut self, value: u8) -> bool {
        self.schedule_register(9, value)
    }

    fn apply_todmid(&mut self, val: u8) {
        if (self.crb & 0x80) != 0 {
            self.tod_alarm = (self.tod_alarm & 0xFF00FF) | ((val as u32) << 8);
        } else {
            self.tod = (self.tod & 0xFF00FF) | ((val as u32) << 8);
            self.tod_halted = true;
        }
    }

    /// Queues TODHI; returns false when capacity is exhausted.
    #[inline]
    pub fn set_todhi(&mut self, value: u8) -> bool {
        self.schedule_register(10, value)
    }

    fn apply_todhi(&mut self, val: u8) {
        if (self.crb & 0x80) != 0 {
            self.tod_alarm = (self.tod_alarm & 0x00FFFF) | ((val as u32) << 16);
        } else {
            self.tod = (self.tod & 0x00FFFF) | ((val as u32) << 16);
            self.tod_halted = true; // Writing TODHI halts counter until TODLO is written
        }
    }

    /// Queues SDR; returns false when capacity is exhausted.
    #[inline]
    pub fn set_sdr(&mut self, value: u8) -> bool {
        self.schedule_register(12, value)
    }

    fn apply_sdr(&mut self, val: u8) {
        self.sdr = val
    }

    /// Queues ICR; returns false when capacity is exhausted.
    #[inline]
    pub fn set_icr(&mut self, value: u8) -> bool {
        self.schedule_register(13, value)
    }

    fn apply_icr(&mut self, val: u8) {
        // SET/CLR bit 7 logic for ICR mask
        if (val & 0x80) != 0 {
            self.icr_mask |= val & 0x1F;
        } else {
            self.icr_mask &= !(val & 0x1F);
        }
        if self.icr_data & self.icr_mask & 0x1f != 0 {
            self.icr_data |= 0x80;
        }
    }

    /// Queues CRA; returns false when capacity is exhausted.
    #[inline]
    pub fn set_cra(&mut self, value: u8) -> bool {
        self.schedule_register(14, value)
    }

    fn apply_cra(&mut self, val: u8) {
        self.cra = val & !0x90;
        if (val & 0x10) != 0 {
            self.ta_counter = self.ta_latch; // Force load
        }
    }

    /// Queues CRB; returns false when capacity is exhausted.
    #[inline]
    pub fn set_crb(&mut self, value: u8) -> bool {
        self.schedule_register(15, value)
    }

    fn apply_crb(&mut self, val: u8) {
        self.crb = val & !0x10;
        if (val & 0x10) != 0 {
            self.tb_counter = self.tb_latch; // Force load
        }
    }

    pub(super) fn apply_register(&mut self, selector: u16, value: u8) {
        match selector {
            0 => self.apply_pra(value),
            1 => self.apply_prb(value),
            2 => self.apply_ddra(value),
            3 => self.apply_ddrb(value),
            4 => self.apply_talo(value),
            5 => self.apply_tahi(value),
            6 => self.apply_tblo(value),
            7 => self.apply_tbhi(value),
            8 => self.apply_todlo(value),
            9 => self.apply_todmid(value),
            10 => self.apply_todhi(value),
            12 => self.apply_sdr(value),
            13 => self.apply_icr(value),
            14 => self.apply_cra(value),
            15 => self.apply_crb(value),
            _ => {}
        }
    }
}
