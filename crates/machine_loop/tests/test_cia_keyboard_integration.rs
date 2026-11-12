#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::A500Config;
use machine_loop::A500Machine;

#[test]
fn keyboard_serial_pins_and_host_acknowledgment_cross_the_motherboard() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cpu.state.set_stopped(true);
    // Deterministic endpoints: this test isolates motherboard wiring.
    machine.keyboard.pins.clock_high = false;
    machine.keyboard.pins.data_high = false;
    machine.step_cck();
    assert!(machine.cia_a.pins.cnt_input_high);
    machine.step_cck();
    assert!(!machine.cia_a.pins.cnt_input_high);
    assert!(!machine.cia_a.pins.sp_input_high);
    assert_eq!(machine.cia_a.sdr(), 0);

    machine.keyboard.pins.clock_high = true;
    machine.keyboard.pins.data_high = true;
    assert!(machine.cia_a.set_cra(0x40));
    // Direction changes reset the active serial shifter before the test drives SP.
    machine.step_cycles(6);
    machine.cia_a.pins.sp_output_high = false;
    machine.step_cycles(6);
    assert!(!machine.keyboard.pins.host_data_high);
    assert!(!machine.cia_a.pins.sp_input_high);
    // Release the output via a committed direction change; no byte is injected.
    assert!(machine.cia_a.set_cra(0));
    machine.step_cycles(6);
    assert!(machine.keyboard.pins.host_data_high);
    assert!(machine.cia_a.pins.sp_input_high);
    assert_eq!(machine.cia_a.sdr(), 0);
}

#[test]
fn test_cia_a_tod_vblank_ticking_and_cia_b_tod_hsync_ticking() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset();

    machine.cia_a.tod = 0;
    machine.cia_b.tod = 0;

    // Step 1 full video frame (to next VBlank transition)
    machine.step_frame();

    // HRM Appendix F: TOD counts a positive pin edge. The source frame boundary
    // has been reached, but the captured rising edge is not delivered yet.
    assert_eq!(machine.cia_a.tod, 0);
    machine.step_cck();
    assert_eq!(machine.cia_a.tod, 1);
    assert_eq!(
        machine.cia_b.tod, 312,
        "one delivered positive edge per PAL line"
    );
}

#[test]
fn test_keyboard_ctrl_amiga_amiga_warm_reset_in_machine_loop() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset();
    let dummy_rom = vec![0x55; 256 * 1024];
    machine
        .physical_memory
        .write_bytes_debug(0xF80000, &dummy_rom)
        .unwrap();

    // Disengage overlay (PRA bit 0 = 1)
    machine.cia_a.set_pra(0x01);
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert!(!machine.physical_memory.is_low_memory_overlay_active());

    // Trigger Ctrl-Amiga-Amiga
    machine.keyboard.key_down(keyboard::SCANCODE_CTRL);
    machine.keyboard.key_down(keyboard::SCANCODE_L_AMIGA);
    machine.keyboard.key_down(keyboard::SCANCODE_R_AMIGA);
    assert!(machine.keyboard.pins.reset_line_asserted);

    // Capture the source assertion, then deliver it to the reset network.
    machine.step_cycles(2);

    // Warm reset re-engages Kickstart overlay
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    assert!(!machine.keyboard.pins.reset_line_asserted);
}
