#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;

fn idle() -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine
}

#[test]
fn fire_is_current_while_drive_sense_waits_for_captured_delivery() {
    let mut machine = idle();
    if let game_ports::PortDevice::Joystick(ref mut joystick) = machine.game_ports.state.port2 {
        joystick.fire1 = true;
    }
    machine.floppy.drives[0].selected = true;
    machine.step_cck();
    assert_eq!(
        machine.cia_a.pins.port_a_input & 0x90,
        0x10,
        "FIRE is current, while drive sense still waits for captured delivery"
    );
    if let game_ports::PortDevice::Joystick(ref mut joystick) = machine.game_ports.state.port2 {
        joystick.fire1 = false;
    }
    machine.step_cck();
    assert_eq!(
        machine.cia_a.pins.port_a_input & 0x90,
        0x80,
        "current FIRE release must preserve the arriving drive status"
    );
}

#[test]
fn blitter_completion_waits_for_delivery_before_requesting_paula() {
    let mut machine = idle();
    machine.agnus.blitter.blit_irq = true;
    machine.step_cck();
    assert_eq!(
        machine.paula.intreqr() & paula::interrupts::IRQ_BLIT,
        0,
        "an Agnus output is not an immediate Paula register mutation"
    );
    machine.step_cck();
    assert_ne!(machine.paula.intreqr() & paula::interrupts::IRQ_BLIT, 0);
}

#[test]
fn horizontal_timing_does_not_directly_tick_cia() {
    let mut machine = idle();
    machine.agnus.hpos = 226;
    let count = machine.cia_b.tod;
    machine.step_cck();
    assert_eq!(
        machine.cia_b.tod, count,
        "the producer step must not directly mutate the recipient counter"
    );
}

#[test]
fn irq_levels_survive_source_release_until_delivery_and_then_release() {
    let mut machine = idle();
    machine.cia_a.icr_mask = 0x10;
    machine.cia_b.icr_mask = 0x10;
    machine.cia_a.trigger_icr(0x10);
    machine.cia_b.trigger_icr(0x10);
    machine.step_cck();
    assert!(machine.paula.pins.int2_high && machine.paula.pins.int6_high);
    assert_eq!(machine.paula.intreqr() & 0x2008, 0);
    machine.cia_a.icr();
    machine.cia_b.icr();
    machine.step_cck();
    assert!(!machine.paula.pins.int2_high && !machine.paula.pins.int6_high);
    assert_eq!(machine.paula.intreqr() & 0x2008, 0x2008);
    assert!(machine.paula.set_intreq(0x2008));
    machine.step_cck();
    assert!(machine.paula.pins.int2_high && machine.paula.pins.int6_high);
    assert_eq!(machine.paula.intreqr() & 0x2008, 0);
}

#[test]
fn tod_edges_and_undelivered_interrupts_resume_after_snapshot() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        machine.agnus.hpos = 225;
        machine.agnus.vpos = if standard == VideoStandard::Pal {
            311
        } else {
            261
        };
        machine.step_cck(); // Falling markers are captured, not yet received.
        assert!(machine.cia_a.pins.tod_high && machine.cia_b.pins.tod_high);
        machine.cia_a.icr_mask = 0x10;
        machine.cia_a.trigger_icr(0x10);
        machine.agnus.blitter.blit_irq = true;
        machine.step_cck(); // Falling levels arrive; rising levels are now in flight.
        assert!(!machine.cia_a.pins.tod_high && !machine.cia_b.pins.tod_high);
        assert!(machine.paula.pins.int2_high && machine.paula.pins.int3_high);
        assert_eq!(machine.paula.intreqr() & 0x0048, 0);
        assert_eq!(machine.cia_a.tod, 0);
        assert_eq!(machine.cia_b.tod, 0);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state = serde_json::from_str::<machine_loop::A500State>(&json).unwrap();
        let mut restored = idle();
        restored.load_state(&state);
        assert_eq!(machine.save_state(), restored.save_state());
        for _ in 0..6 {
            machine.step_cck();
            restored.step_cck();
            assert_eq!(machine.save_state(), restored.save_state());
            assert_eq!(machine.cia_a.tod, 1, "held high must not count again");
            assert_eq!(machine.cia_b.tod, 1);
            assert_eq!(machine.paula.intreqr() & 0x0048, 0x0048);
        }
    }
}

#[test]
fn connector_control_and_sense_form_separate_captured_hops() {
    let mut machine = idle();
    machine.cia_b.ddrb = 0xff;
    machine.cia_b.prb = 0x7f; // HRM: establish motor data before selecting a drive.
    machine.step_cycles(2);
    assert_eq!(machine.floppy.pins.control_input, 0x7f);
    assert!(!machine.floppy.drives[0].motor_on);
    machine.cia_b.prb = 0x77; // Assert motor and select DF0, without a step edge.
    machine.step_cck();
    assert_eq!(machine.floppy.pins.control_input, 0x7f);
    assert!(!machine.floppy.drives[0].selected);
    machine.step_cck();
    assert_eq!(machine.floppy.pins.control_input, 0x77);
    assert!(machine.floppy.drives[0].selected);
    assert_ne!(machine.cia_a.pins.port_a_input & 0x10, 0);
    machine.step_cck();
    assert_eq!(machine.cia_a.pins.port_a_input & 0x10, 0);
}

#[test]
fn keyboard_reset_does_not_bypass_capture() {
    let mut machine = idle();
    machine.keyboard.pins.reset_line_asserted = true;
    machine.step_cck();
    assert_eq!(machine.cck, 1);
    assert!(
        machine.keyboard_reset_asserted,
        "reset is captured for the next CCK"
    );
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str::<machine_loop::A500State>(&json).unwrap();
    let mut restored = idle();
    restored.load_state(&state);
    assert!(
        restored.keyboard_reset_asserted,
        "load must retain the pending reset"
    );
    machine.keyboard.pins.reset_line_asserted = false; // Captured assertion must survive.
    restored.keyboard.pins.reset_line_asserted = false;
    machine.step_cck();
    restored.step_cck();
    assert_eq!(
        machine.cck, 0,
        "the delivered reset assertion must warm-reset once"
    );
    assert!(!machine.keyboard_reset_asserted);
    assert_eq!(machine.save_state(), restored.save_state());
}

#[test]
fn digital_pot_buttons_are_current_while_counter_delivery_and_snapshots_are_preserved() {
    let mut machine = idle();
    machine.game_ports.set_mouse_buttons(false, true, false);
    machine.game_ports.state.pot0dat = 0x1234;
    machine.game_ports.state.pot1dat = 0x5678;
    machine.step_cck();
    assert_eq!(
        machine.paula.pins.pot_input_high & 0x0400,
        0,
        "current buttons arrive without an extra captured hop"
    );
    assert_eq!(machine.paula.potgor() & 0x0400, 0);
    assert_eq!((machine.paula.pot0dat(), machine.paula.pot1dat()), (0, 0));

    // Save a new source level before sampling it; loading must not deliver it.
    machine.game_ports.set_mouse_buttons(false, false, false);
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str::<machine_loop::A500State>(&json).unwrap();
    let mut restored = idle();
    restored.load_state(&state);
    assert_eq!(restored.paula.pins.pot_input_high & 0x0400, 0);
    for target in [&mut machine, &mut restored] {
        target.step_cck();
        assert_ne!(target.paula.pins.pot_input_high & 0x0400, 0);
        assert_ne!(target.paula.potgor() & 0x0400, 0);
        assert_eq!(
            (target.paula.pot0dat(), target.paula.pot1dat()),
            (0x1234, 0x5678),
            "legacy counter delivery keeps its existing phase"
        );
    }
    assert_eq!(machine.save_state(), restored.save_state());
}
