use denise::{Denise, DeniseModel};

#[test]
fn quadrature_wraps_both_axes_and_joytest_preserves_pin_bits() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    // One positive X cycle and negative Y cycle, simultaneously on both ports.
    for pins in [0x09, 0x00, 0x06, 0x0f] {
        denise.receive_game_port_pins([pins; 2]);
        denise.step_cck();
    }
    assert_eq!(denise.joy0dat(), 0xfc04);
    assert_eq!(denise.joy1dat(), 0xfc04);
    denise.receive_game_port_pins([0x09, 0x06]);
    denise.step_cck();
    let low_bits = [denise.joy0dat() & 0x0303, denise.joy1dat() & 0x0303];
    assert!(denise.set_joytest(0x5678));
    denise.step_cck();
    assert_eq!(denise.joy0dat(), 0x5478 | low_bits[0]);
    assert_eq!(denise.joy1dat(), 0x5478 | low_bits[1]);
    denise.receive_game_port_pins([0x0f; 2]);
    denise.step_cck();
    assert_eq!(denise.joy0dat(), 0x5878);
}

#[test]
fn every_joystick_contact_combination_is_visible_through_xor_bits() {
    for pressed in 0..16u8 {
        let mut denise = Denise::new(DeniseModel::Ocs8362);
        denise.receive_game_port_pins([!pressed & 0x0f; 2]);
        denise.step_cck();
        for value in [denise.joy0dat(), denise.joy1dat()] {
            assert_eq!(value & 0x0200 != 0, pressed & 4 != 0);
            assert_eq!((value ^ (value << 1)) & 0x0200 != 0, pressed & 1 != 0);
            assert_eq!(value & 0x0002 != 0, pressed & 8 != 0);
            assert_eq!((value ^ (value << 1)) & 0x0002 != 0, pressed & 2 != 0);
        }
    }
}

#[test]
fn held_pins_preserve_joytest_counts_until_the_next_transition() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    // X forward to phase 3, Y backward to phase 1.
    for pins in [0x09, 0x00, 0x06] {
        denise.receive_game_port_pins([pins; 2]);
        denise.step_cck();
    }
    assert_eq!(denise.joy0dat(), 0xfd03);
    assert!(denise.set_joytest(0xfcfc));
    denise.step_cck();
    for _ in 0..32 {
        denise.receive_game_port_pins([0x06; 2]);
        denise.step_cck();
        assert_eq!(denise.joy0dat(), 0xfdff);
        assert_eq!(denise.joy1dat(), 0xfdff);
    }
    let mut restored = denise.clone();
    for target in [&mut denise, &mut restored] {
        // Port 1 continues across X wrap; port 2 reverses both axes.
        target.receive_game_port_pins([0x0f, 0x00]);
        target.step_cck();
        assert_eq!(target.joy0dat(), 0xfc00);
        assert_eq!(target.joy1dat(), 0xfefe);
        target.reset();
        target.receive_game_port_pins([0x06; 2]);
        target.step_cck();
        assert_eq!(target.joy0dat(), 0x01ff);
        assert_eq!(target.joy1dat(), 0x01ff);
    }
    assert_eq!(denise, restored);
}
