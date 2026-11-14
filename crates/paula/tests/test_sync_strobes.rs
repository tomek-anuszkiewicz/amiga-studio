use paula::Paula;

fn receive(paula: &mut Paula, address: u16) {
    paula.pins.rga_strobe = Some(address);
    paula.step_cck();
}

#[test]
fn all_line_strobes_capture_and_emit_audio_requests() {
    // Approved RGA model: all three line markers anchor the same DMAL frame.
    for address in [0x038, 0x03a, 0x03c] {
        let mut paula = Paula::default();
        paula.sync.cck_since_horizontal = 80;
        let channel = &mut paula.audio.channels[0];
        channel.dma_enabled = true;
        channel.dma_request = true;
        channel.restart_request = true;
        receive(&mut paula, address);
        assert_eq!(paula.sync.cck_since_horizontal, 0, "address {address:#05x}");
        assert_eq!(paula.sync.dmal_frame, 0x0300);
        assert!(!paula.pins.dmal_output_high);
        for elapsed in 1..=14 {
            // The additional long-line marker must not recapture requests or phase.
            if elapsed == 2 {
                paula.audio.channels[0].dma_request = false;
                paula.pins.rga_strobe = Some(0x03e);
            }
            paula.step_cck();
            assert_eq!(paula.sync.cck_since_horizontal, elapsed);
            assert_eq!(paula.sync.dmal_frame, 0x0300);
            assert_eq!(paula.pins.dmal_output_high, matches!(elapsed, 6 | 7));
        }
    }
}

#[test]
fn equalization_entry_requests_vertb_once_and_wins_a_coincident_clear() {
    let mut paula = Paula::default();
    assert!(paula.set_intena(0xc020));
    assert!(paula.set_intreq(0x0020));
    receive(&mut paula, 0x038);
    assert_eq!(paula.intreqr() & 0x0020, 0x0020);
    assert_eq!(paula.pending_interrupt_level(), 3);

    assert!(paula.set_intreq(0x0020));
    paula.step_cck();
    assert_eq!(paula.intreqr() & 0x0020, 0);
    for address in [0x038, 0x03e, 0x038, 0x03a, 0x03e, 0x03c] {
        receive(&mut paula, address);
        assert_eq!(paula.intreqr() & 0x0020, 0, "address {address:#05x}");
        assert_eq!(paula.pending_interrupt_level(), 0);
    }
    receive(&mut paula, 0x038);
    assert_eq!(paula.intreqr() & 0x0020, 0x0020);
    assert!(paula.set_intreq(0x0020));
    receive(&mut paula, 0x038);
    assert_eq!(paula.intreqr() & 0x0020, 0, "held phase must not reassert");

    paula.reset();
    assert_eq!(paula.sync.cck_since_horizontal, 0);
    assert_eq!(paula.sync.dmal_frame, 0);
    receive(&mut paula, 0x038);
    assert_eq!(paula.intreqr() & 0x0020, 0x0020);
}
