#![allow(clippy::unwrap_used)]

use config::{A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn idle(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    // Keep the short test interval away from horizontal and vertical strobes.
    machine.agnus.hpos = 40;
    machine.agnus.vpos = 100;
    machine
}

fn assert_level_delivery(machine: &A500Machine) {
    assert!(machine.agnus.pins.incoming_dmal_high.is_some());
    assert!(machine.denise.pins.incoming.is_some());
    assert!(machine.paula.pins.incoming.is_some());
    for cia in [&machine.cia_a, &machine.cia_b] {
        assert!(cia.pins.incoming.is_some());
        assert!(cia.pins.incoming_port_a.is_some());
    }
    assert!(machine.cia_a.pins.incoming_flag_high.is_none());
    assert!(machine.cia_b.pins.incoming_flag_high.is_some());
    assert!(machine.floppy.pins.incoming.is_some());
    assert!(machine.keyboard.pins.incoming_host_data_high.is_some());
    assert!(machine.game_ports.pins.incoming.is_some());
}

#[test]
fn unchanged_levels_remain_delivered_with_buttons_held() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.game_ports.set_mouse_buttons(true, true, true);
        machine
            .game_ports
            .set_joystick(true, false, false, true, true, true);
        machine.cia_b.ddrb = 0xff;
        machine.cia_b.prb = 0x77; // Select DF0; allow both connector hops to settle.
        machine.step_cycles(4);
        for _ in 0..16 {
            assert_level_delivery(&machine);
            assert_eq!(machine.cia_a.pins.port_a_input & 0xc0, 0);
            assert_eq!(machine.paula.potgor() & 0x4500, 0);
            assert!(machine.floppy.drives[0].selected);
            machine.step_cck();
        }
    }
}

#[test]
fn identical_sync_addresses_are_delivered_as_distinct_events() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        for _ in 0..2 {
            // Two producer strobes on adjacent CCKs must both reach the receivers.
            machine.agnus.hpos = 0;
            machine.step_cck();
            assert_eq!(machine.agnus.pins.rga_strobe, Some(0x03c));
            assert_eq!(
                machine.denise.pins.incoming.unwrap().rga_strobe,
                Some(0x03c)
            );
            assert_eq!(machine.paula.pins.incoming.unwrap().rga_strobe, Some(0x03c));
        }
        assert_eq!(machine.denise.sync.vpos, 0, "first strobe received");
        machine.step_cck();
        assert_eq!(machine.denise.sync.vpos, 1, "second strobe received");
        assert_eq!(machine.paula.sync.cck_since_horizontal, 0);
        assert_level_delivery(&machine);
        machine.step_cck();
        assert_eq!(machine.denise.sync.vpos, 1, "no extra strobe invented");
        assert_eq!(machine.paula.sync.cck_since_horizontal, 1);
    }
}

#[test]
fn held_interrupt_reasserts_after_clear_with_unchanged_pin_levels() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.cia_a.icr_mask = 0x01;
        machine.cia_a.trigger_icr(0x01);
        machine.step_cycles(2);
        assert!(!machine.paula.pins.int2_high);
        assert_level_delivery(&machine);

        assert!(machine.paula.set_intreq(0x0008));
        machine.step_cck();
        assert_eq!(machine.paula.intreqr() & 0x0008, 0x0008);
        assert_level_delivery(&machine);

        machine.cia_a.icr();
        machine.step_cck();
        assert!(!machine.paula.pins.int2_high, "release is still in flight");
        assert!(machine.paula.pins.incoming.unwrap().int2_high);
        assert!(machine.paula.set_intreq(0x0008));
        machine.step_cck();
        assert!(machine.paula.pins.int2_high);
        assert_eq!(machine.paula.intreqr() & 0x0008, 0);
        assert_level_delivery(&machine);
    }
}

#[test]
fn restored_quiet_inputs_accept_new_edges_at_the_original_phase() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.step_cycles(4);
        assert_level_delivery(&machine);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state = serde_json::from_str::<A500State>(&json).unwrap();
        let mut restored = idle(standard);
        restored.load_state(&state);

        for target in [&mut machine, &mut restored] {
            target.keyboard.pins.data_high = false;
            target.keyboard.pins.clock_high = false;
            target.floppy.pins.index_high = false;
            target.game_ports.set_mouse_buttons(true, true, false);
            target.step_cck();
            assert!(target.cia_a.pins.sp_input_high);
            assert!(target.cia_a.pins.cnt_input_high);
            assert!(target.cia_b.pins.flag_high);
            assert_eq!(target.cia_a.pins.port_a_input & 0x40, 0);
            assert_eq!(target.paula.potgor() & 0x0400, 0);
            // Releasing sources cannot cancel the changes already in flight.
            target.keyboard.pins.data_high = true;
            target.keyboard.pins.clock_high = true;
            target.floppy.pins.index_high = true;
            target.step_cck();
            assert!(!target.cia_a.pins.sp_input_high);
            assert!(!target.cia_a.pins.cnt_input_high);
            assert!(!target.cia_b.pins.flag_high);
            assert_eq!(target.cia_b.icr_debug() & 0x10, 0x10);
            target.step_cck();
            assert!(target.cia_a.pins.sp_input_high);
            assert!(target.cia_a.pins.cnt_input_high);
            assert!(target.cia_b.pins.flag_high);
            assert_level_delivery(target);
        }
        assert_eq!(machine.save_state(), restored.save_state());
    }
}
