//! Keyboard/CIA production pins, without firmware or an SDR injection.
#![allow(clippy::panic)]

use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;

fn acknowledge(machine: &mut A500Machine) {
    assert!(machine.cia_a.set_cra(0x40));
    machine.step_cycles(330);
    assert!(!machine.keyboard.pins.host_data_high);
    assert!(machine.cia_a.set_cra(0));
    machine.step_cycles(10);
}

fn receive(machine: &mut A500Machine) -> u8 {
    for _ in 0..4000 {
        machine.step_cck();
        if machine.cia_a.icr_debug() & 8 != 0 {
            assert_eq!(machine.cia_a.icr() & 0x88, 0x88);
            let value = machine.cia_a.sdr();
            machine.step_cycles(100); // End of the keyboard's final recovery portion.
            return value;
        }
    }
    panic!("CIA received no complete keyboard byte");
}

#[test]
fn startup_keys_and_partial_serial_state_use_the_captured_wire_path() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        assert!(machine.cia_a.set_icr(0x88));
        machine.step_cycles(400);
        assert_eq!(machine.cia_a.icr_debug() & 8, 0);
        acknowledge(&mut machine); // Reinitialize the CIA shifter after the sync bit.
        for value in [0x04, 0x02] {
            assert_eq!(receive(&mut machine), value);
            acknowledge(&mut machine);
        }
        machine.keyboard.key_down(0x35);
        machine.step_cycles(700); // In-flight bits, no complete key yet.
        assert_eq!(machine.cia_a.icr_debug() & 8, 0);
        let snapshot = machine.save_state();
        let mut restored = A500Machine::new(A500Config::bare_512k(standard));
        restored.load_state(&snapshot);
        assert_eq!(receive(&mut machine), 0x95);
        assert_eq!(receive(&mut restored), 0x95);
        assert_eq!(machine.keyboard, restored.keyboard);
        assert_eq!(machine.cia_a, restored.cia_a);
        acknowledge(&mut machine);
        machine.keyboard.key_up(0x35);
        assert_eq!(receive(&mut machine), 0x94);
    }
}
