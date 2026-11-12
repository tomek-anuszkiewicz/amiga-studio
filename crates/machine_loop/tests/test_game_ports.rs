use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;

#[test]
fn queued_mouse_motion_and_digital_pot_drive_resume_through_production_pins() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        machine.game_ports.apply_mouse_delta(260, -257);
        machine.step_cycles(33);
        assert_eq!(machine.denise.joy0dat(), 0xdf21);
        let state = machine.save_state();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&state);
        for target in [&mut machine, &mut restored] {
            target.step_cycles(227);
            assert_eq!(target.denise.joy0dat(), 0xff04);
            assert!(target.paula.set_potgo(0xaa00)); // All four outputs low.
            target.step_cycles(2);
            assert_eq!(target.paula.potgor(), 0);
            assert!(target.paula.set_potgo(0)); // Release all four inputs.
            target.step_cycles(2);
            assert_eq!(target.paula.potgor(), 0x5500);
        }
        assert_eq!(machine.save_state(), restored.save_state());
    }
}
