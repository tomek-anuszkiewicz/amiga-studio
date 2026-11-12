//! 8520 count sources, reloads and PB6/PB7 timer waveforms (HRM Appendix F).

use crate::registers::{
    BIT_CONTROL_ONE_SHOT, BIT_CONTROL_START, BIT_CONTROL_TOGGLE, BIT_CRA_COUNT_CNT,
    BIT_CRB_INPUT_MODE_SHIFT, BIT_ICR_TIMER_A, BIT_ICR_TIMER_B, CRB_COUNT_CNT, CRB_COUNT_ECLOCK,
    CRB_COUNT_TA_UNDERFLOW, CRB_COUNT_TA_UNDERFLOW_CNT_HIGH, MASK_CRB_INPUT_MODE,
};
use crate::{Cia, CCK_PER_ECLOCK};

fn count(counter: &mut u16, latch: u16, control: &mut u8, clock: bool) -> bool {
    if !clock || *control & BIT_CONTROL_START == 0 {
        return false;
    }
    if *counter != 0 {
        *counter = counter.wrapping_sub(1);
        return false;
    }
    *counter = latch;
    if *control & BIT_CONTROL_ONE_SHOT != 0 {
        *control &= !BIT_CONTROL_START;
    }
    true
}

fn expire_pulse(remaining: &mut u8, output: &mut bool, control: u8) {
    if *remaining != 0 {
        *remaining -= 1;
        if *remaining == 0 && control & BIT_CONTROL_TOGGLE == 0 {
            *output = false;
        }
    }
}

fn underflow_output(output: &mut bool, pulse: &mut u8, control: u8) {
    if control & BIT_CONTROL_TOGGLE != 0 {
        *output = !*output;
    } else {
        *output = true;
        *pulse = CCK_PER_ECLOCK;
    }
}

impl Cia {
    pub(super) fn start_timer_a_output(&mut self) {
        if self.cra & BIT_CONTROL_TOGGLE != 0 {
            self.timer_a_output = true;
        }
        self.prb_mutated = true;
    }

    pub(super) fn start_timer_b_output(&mut self) {
        if self.crb & BIT_CONTROL_TOGGLE != 0 {
            self.timer_b_output = true;
        }
        self.prb_mutated = true;
    }

    pub(super) fn step_timers(&mut self, e_rising: bool, cnt_rising: bool) {
        let previous = self.port_b_output_pins();
        expire_pulse(&mut self.timer_a_pulse, &mut self.timer_a_output, self.cra);
        expire_pulse(&mut self.timer_b_pulse, &mut self.timer_b_output, self.crb);
        let clock_a = if self.cra & BIT_CRA_COUNT_CNT == 0 {
            e_rising
        } else {
            cnt_rising
        };
        let underflow_a = count(&mut self.ta_counter, self.ta_latch, &mut self.cra, clock_a);
        if underflow_a {
            self.trigger_icr(BIT_ICR_TIMER_A);
            underflow_output(&mut self.timer_a_output, &mut self.timer_a_pulse, self.cra);
            self.clock_serial_output();
        }
        let clock_b = match (self.crb >> BIT_CRB_INPUT_MODE_SHIFT) & MASK_CRB_INPUT_MODE {
            CRB_COUNT_ECLOCK => e_rising,
            CRB_COUNT_CNT => cnt_rising,
            CRB_COUNT_TA_UNDERFLOW => underflow_a,
            CRB_COUNT_TA_UNDERFLOW_CNT_HIGH => underflow_a && self.pins.cnt_input_high,
            _ => false,
        };
        if count(&mut self.tb_counter, self.tb_latch, &mut self.crb, clock_b) {
            self.trigger_icr(BIT_ICR_TIMER_B);
            underflow_output(&mut self.timer_b_output, &mut self.timer_b_pulse, self.crb);
        }
        self.prb_mutated |= previous != self.port_b_output_pins();
    }
}
