//! HRM Appendix F: timer modes, serial transfer and parallel handshaking.

use cia::Cia;

fn capture(cia: &mut Cia) {
    cia.step_cck();
    while cia.eclock_phase != 2 {
        cia.step_cck();
    }
}

fn e_tick(cia: &mut Cia) {
    cia.step_cck();
    while cia.eclock_phase != 0 {
        cia.step_cck();
    }
}

fn cnt_edge(cia: &mut Cia, data: bool) {
    cia.receive_serial_pins(data, false);
    cia.step_cck();
    cia.receive_serial_pins(data, true);
    cia.step_cck();
}

#[test]
fn timer_a_selects_positive_cnt_edges_and_reloads_after_zero() {
    let mut cia = Cia::new();
    cia.ta_counter = 2;
    cia.ta_latch = 2;
    assert!(cia.set_cra(0x21));
    capture(&mut cia);
    for _ in 0..4 {
        e_tick(&mut cia);
    }
    assert_eq!(cia.ta_counter, 2);
    for expected in [1, 0, 2] {
        cnt_edge(&mut cia, true);
        assert_eq!(cia.ta_counter, expected);
    }
    assert_eq!(cia.icr(), 1);
    cia.receive_serial_pins(true, true);
    cia.step_cck();
    assert_eq!(cia.ta_counter, 2, "held CNT is not another count");
}

#[test]
fn all_timer_b_clock_sources_include_cnt_gated_cascade() {
    for (control, after_low, after_edge, after_high) in [
        (0x01, 1, 1, 0),
        (0x21, 4, 3, 3),
        (0x41, 1, 1, 0),
        (0x61, 4, 4, 3),
    ] {
        let mut cia = Cia::new();
        cia.ta_counter = 0;
        cia.ta_latch = 0;
        cia.tb_counter = 4;
        cia.tb_latch = 4;
        assert!(cia.set_cra(1));
        assert!(cia.set_crb(control));
        capture(&mut cia);
        cia.receive_serial_pins(true, false);
        for _ in 0..3 {
            e_tick(&mut cia);
        }
        assert_eq!(cia.tb_counter, after_low, "control {control:x}");
        cia.receive_serial_pins(true, true);
        cia.step_cck();
        assert_eq!(cia.tb_counter, after_edge, "control {control:x}");
        e_tick(&mut cia);
        assert_eq!(cia.tb_counter, after_high, "control {control:x}");
    }
}

#[test]
fn timer_outputs_override_input_direction_and_support_toggle_and_pulse() {
    let mut cia = Cia::new();
    cia.ta_counter = 1;
    cia.ta_latch = 1;
    cia.tb_counter = 1;
    cia.tb_latch = 1;
    assert!(cia.set_cra(0x07));
    assert!(cia.set_crb(0x07));
    capture(&mut cia);
    assert_eq!(cia.ddrb(), 0);
    assert_eq!(cia.prb_debug() & 0xc0, 0xc0);
    e_tick(&mut cia);
    assert_eq!(cia.prb_debug() & 0xc0, 0xc0);
    e_tick(&mut cia);
    assert_eq!(cia.port_b_output_pins() & 0xc0, 0);
    assert_eq!(cia.prb_debug() & 0xc0, 0);
    e_tick(&mut cia);
    e_tick(&mut cia);
    assert_eq!(cia.prb_debug() & 0xc0, 0xc0);

    cia.reset();
    cia.ta_counter = 0;
    cia.tb_counter = 0;
    assert!(cia.set_cra(0x0b));
    assert!(cia.set_crb(0x0b));
    capture(&mut cia);
    assert_eq!(cia.port_b_output_pins() & 0xc0, 0);
    e_tick(&mut cia);
    assert_eq!(cia.cra() & 1, 0);
    assert_eq!(cia.crb() & 1, 0);
    for _ in 0..5 {
        assert_eq!(cia.port_b_output_pins() & 0xc0, 0xc0);
        cia.step_cck();
    }
    assert_eq!(cia.port_b_output_pins() & 0xc0, 0);
    assert_eq!(cia.icr() & 3, 3);
}

#[test]
fn serial_input_publishes_only_a_full_msb_first_byte() {
    let mut cia = Cia::new();
    assert!(cia.set_icr(0x88));
    capture(&mut cia);
    for value in [0xa6, 0x35] {
        let old = cia.sdr();
        for bit in (0..8).rev() {
            cnt_edge(&mut cia, value & (1 << bit) != 0);
            if bit != 0 {
                assert_eq!(cia.sdr(), old);
                assert!(!cia.irq_pending());
            }
        }
        assert_eq!(cia.sdr(), value);
        assert_eq!(cia.icr(), 0x88);
    }
}

#[test]
fn serial_output_buffers_the_next_byte_without_replacing_active_bits() {
    let mut cia = Cia::new();
    cia.ta_latch = 1;
    cia.ta_counter = 1;
    assert!(cia.set_cra(0x40));
    capture(&mut cia);
    assert!(cia.set_sdr(0xa6));
    capture(&mut cia);
    assert!(cia.set_cra(0x41));
    capture(&mut cia);
    let mut bits = Vec::new();
    let mut serial_interrupts = 0;
    for _ in 0..400 {
        let clock_before = cia.pins.cnt_output_high;
        cia.step_cck();
        if !clock_before && cia.pins.cnt_output_high {
            bits.push(cia.pins.sp_output_high);
            if bits.len() == 3 {
                assert!(cia.set_sdr(0x35));
            }
        }
        if cia.icr() & 8 != 0 {
            serial_interrupts += 1;
        }
    }
    let expected: Vec<bool> = [0xa6, 0x35]
        .into_iter()
        .flat_map(|value| (0..8).rev().map(move |bit| value & (1 << bit) != 0))
        .collect();
    assert_eq!(bits, expected);
    assert_eq!(serial_interrupts, 2);
    assert!(cia.pins.cnt_output_high);
    assert!(cia.set_cra(0));
    capture(&mut cia);
    assert!(cia.pins.sp_output_high);
    cnt_edge(&mut cia, false);
    assert_eq!(cia.icr() & 8, 0, "direction change discards partial state");
}

#[test]
fn port_access_strobes_pc_on_third_e_cycle_and_flag_is_edge_sensitive() {
    for write in [false, true] {
        let mut cia = Cia::new();
        assert_eq!(cia.prb_debug(), 0xff);
        for _ in 0..4 {
            e_tick(&mut cia);
            assert!(cia.pins.pc_high);
        }
        if write {
            assert!(cia.set_prb(0xa5));
            capture(&mut cia);
        } else {
            assert_eq!(cia.prb(), 0xff);
        }
        for high in [true, true, false, true] {
            e_tick(&mut cia);
            assert_eq!(cia.pins.pc_high, high);
        }
        cia.receive_flag_pin(false);
        assert_eq!(cia.icr(), 0x10);
        cia.receive_flag_pin(false);
        assert_eq!(cia.icr(), 0);
        cia.receive_flag_pin(true);
        cia.receive_flag_pin(false);
        assert_eq!(cia.icr(), 0x10);
    }
}

#[test]
fn tod_alarm_is_write_only_and_counter_wrap_preserves_the_read_latch() {
    let mut cia = Cia::new();
    assert!(cia.set_todhi(0xff));
    assert!(cia.set_todmid(0xff));
    assert!(cia.set_todlo(0xfe));
    capture(&mut cia);
    assert_eq!(cia.tod, 0xfffffe);
    assert!(cia.set_crb(0x80));
    capture(&mut cia);
    assert!(cia.set_todhi(0));
    assert!(cia.set_todmid(0));
    assert!(cia.set_todlo(0));
    assert!(cia.set_icr(0x84));
    capture(&mut cia);
    assert_eq!(cia.todhi(), 0xff, "alarm selection does not affect reads");
    capture(&mut cia);
    for expected in [0xffffff, 0] {
        cia.receive_tod_pin(false);
        cia.receive_tod_pin(true);
        assert_eq!(cia.tod, expected);
        cia.receive_tod_pin(true);
        assert_eq!(cia.tod, expected, "a held input does not count again");
    }
    assert_eq!(cia.icr_debug(), 0x84);
    assert_eq!(cia.todmid(), 0xff);
    assert_eq!(cia.todlo(), 0xfe);
    capture(&mut cia);
    assert_eq!(cia.todlo(), 0);
    assert_eq!(cia.icr(), 0x84);
    assert!(!cia.irq_pending());
}

#[test]
fn every_interrupt_source_retains_masked_requests_and_acknowledges_together() {
    for source in [1u8, 2, 4, 8, 16] {
        let mut cia = Cia::new();
        cia.trigger_icr(0x1f);
        assert_eq!(cia.icr_debug(), 0x1f);
        assert!(!cia.irq_pending());
        assert!(cia.set_icr(0x80 | source));
        capture(&mut cia);
        assert_eq!(cia.icr_debug(), 0x9f);
        assert!(cia.set_icr(source));
        capture(&mut cia);
        assert_eq!(cia.icr(), 0x9f, "mask clear does not acknowledge a request");
        assert_eq!(cia.icr_debug(), 0);
        cia.trigger_icr(source);
        assert_eq!(cia.icr(), source);
        cia.reset();
        assert_eq!(cia.icr_debug(), 0);
        assert_eq!(cia.icr_mask, 0);
    }
}
