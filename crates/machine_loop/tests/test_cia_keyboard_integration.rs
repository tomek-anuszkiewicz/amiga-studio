#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::A500Config;
use machine_loop::A500Machine;

#[test]
fn test_keyboard_scancode_delivery_to_cia_and_level2_irq() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset();

    // Enable CIA-A SDR interrupt in ICR: $80 | 0x08 = $88
    machine.cia_a.write_register(0xD, 0x88);
    // Enable Level 2 PORTS interrupt in Paula INTENA: SET(0x8000) | INTEN(0x4000) | PORTS(0x0008) = $C008
    machine.paula.write_intena(0xC008);

    assert_eq!(machine.cpu.state.ipl, 0);

    // Press key 'A' (Amiga raw position $20)
    machine.keyboard.key_down(0x20);

    // Step machine by 5 Color Clocks to allow subsystem stepping and keyboard delivery
    machine.step_cycles(5);

    // Verify scancode was shifted into CIA-A SDR
    let expected_sdr = (0x20 << 1) & 0xFE; // $40
    assert_eq!(machine.cia_a.sdr, expected_sdr);

    // Verify CIA-A requested interrupt and Paula escalated to IPL 2 (PORTS)
    assert!(machine.cia_a.irq_pending());
    assert_eq!(machine.cpu.state.ipl, 2);

    // Emulate OS driver: read SDR and acknowledge by setting CRA bit 6 (SPMODE = 1)
    let sdr_read = machine.cia_a.read_register(0xC);
    assert_eq!(sdr_read, expected_sdr);
    // Reading ICR clears CIA-A IRQ
    let _ = machine.cia_a.read_register(0xD);
    assert!(!machine.cia_a.irq_pending());

    // Pulse KDAT low (CRA bit 6 = 1)
    machine.cia_a.write_register(0xE, 0x40);
    machine.step_cycles(5);

    // Restore CRA to input mode (SPMODE = 0)
    machine.cia_a.write_register(0xE, 0x00);
    machine.step_cycles(5);

    // Keyboard should have acknowledged and be back in Idle state
    assert_eq!(
        machine.keyboard.transmission_state,
        keyboard::KeyboardTransmissionState::Idle
    );
}

#[test]
fn test_cia_a_tod_vblank_ticking_and_cia_b_tod_hsync_ticking() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset();

    machine.cia_a.tod = 0;
    machine.cia_b.tod = 0;

    // Step 1 full video frame (to next VBlank transition)
    machine.step_frame();

    // CIA-A TOD must have ticked on VBlank transition (50 Hz tick)
    assert_eq!(machine.cia_a.tod, 1);

    // CIA-B TOD must have ticked on every horizontal scanline (~312 PAL scanlines)
    assert!(
        machine.cia_b.tod >= 310 && machine.cia_b.tod <= 314,
        "Expected ~312 scanlines, got {}",
        machine.cia_b.tod
    );
}

#[test]
fn test_keyboard_ctrl_amiga_amiga_warm_reset_in_machine_loop() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset();
    let dummy_rom = [0x55; 512 * 1024];
    machine
        .physical_memory
        .write_bytes_debug(0xF80000, &dummy_rom);

    // Disengage overlay (PRA bit 0 = 1)
    machine.cia_a.write_register(0x0, 0x01);
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert!(!machine.physical_memory.is_low_memory_overlay_active());

    // Trigger Ctrl-Amiga-Amiga
    machine.keyboard.key_down(keyboard::SCANCODE_CTRL);
    machine.keyboard.key_down(keyboard::SCANCODE_L_AMIGA);
    machine.keyboard.key_down(keyboard::SCANCODE_R_AMIGA);
    assert!(machine.keyboard.reset_line_asserted);

    // Step 1 CCK in machine loop: processes keyboard reset line
    machine.step_cck();

    // Warm reset re-engages Kickstart overlay
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    assert!(!machine.keyboard.reset_line_asserted);
}
