#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for Amiga 500 machine-wide reset sequencing (Step 2.5)
//!
//! Validates:
//! - Cold reset: zeroes all RAM buffers, resets custom chips to power-on defaults,
//!   zeroes CPU data/address registers, and initialises vectors.
//! - Warm reset: preserves RAM buffers intact, resets custom chips, preserves CPU
//!   data/address registers intact, and reloads vectors.
//! - M68000 RESET instruction ($4E70) execution: pulses external reset line, resetting
//!   custom chips and re-engaging Gary overlay while leaving RAM and CPU registers/PC untouched.
//! - M68000 RESET instruction privilege violation: traps to Vector 8 when executed in user mode.
//! - Hardware keyboard reset line (Ctrl-Amiga-Amiga): automatically executes warm reset.
//! - Boot overlay engagement: verifies Kickstart overlay vs synthetic test mode fallback.

use config::A500Config;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_cold_reset_full_flow() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);
    machine.physical_memory.map_chip_ram_to_low_memory();

    // 1. Pre-populate RAM buffers
    let _ = machine.memory_bus().write_byte(0x001000, 0xAA);
    let _ = machine.memory_bus().write_byte(0x002000, 0xBB);
    assert_eq!(machine.physical_memory.read_byte_debug(0x001000), 0xAA);
    assert_eq!(machine.physical_memory.read_byte_debug(0x002000), 0xBB);

    // 2. Modify custom chip registers
    machine.agnus.dma.write_dmacon(0x8200); // Enable master DMA
    machine.denise.color[0] = 0x0F00; // Red background
    machine.paula.write_intena(0x8080); // Enable Audio 0 IRQ
    machine.cia_a.write_register(0x2, 0x03); // DDRA bits 0-1
    machine.cia_a.write_register(0x0, 0x01); // PRA bit 0

    // 3. Modify CPU registers and state
    machine.cpu.state.set_d_long(0, 0x12345678);
    machine.cpu.state.set_d_long(1, 0x87654321);
    machine.cpu.state.set_a_long(0, 0x00020000);
    machine.cpu.state.set_a_long(1, 0x00030000);

    // 4. Advance machine clocks
    machine.step_cycles(1000);
    assert_eq!(machine.cck, 1000);

    // 5. Trigger Cold Reset
    machine.reset();

    // Verify clock counter reset
    assert_eq!(
        machine.cck, 0,
        "CCK counter must be reset to 0 on cold reset"
    );

    // Verify boot overlay re-engaged and Chip RAM wiped to zero
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Boot overlay must be re-engaged on cold reset"
    );
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert_eq!(
        machine.physical_memory.read_byte_debug(0x001000),
        0x00,
        "Chip RAM must be zeroed on cold reset"
    );
    assert_eq!(
        machine.physical_memory.read_byte_debug(0x002000),
        0x00,
        "Chip RAM must be zeroed on cold reset"
    );

    // Verify custom chips reset to power-on defaults
    assert_eq!(
        machine.agnus.dma.dmacon, 0x0000,
        "DMACON must be reset to 0"
    );
    assert_eq!(
        machine.denise.color[0], 0x0000,
        "Palette must be reset to 0"
    );
    assert_eq!(
        machine.paula.interrupts.intena, 0x0000,
        "INTENA must be reset to 0"
    );
    assert_eq!(machine.cia_a.ddra, 0x00, "CIA-A DDRA must be reset to 0");

    // Verify CPU registers zeroed
    for i in 0..8 {
        assert_eq!(
            machine.cpu.state.d_long(i),
            0,
            "CPU data register D{i} must be cleared to 0 on cold reset"
        );
    }
    for i in 0..7 {
        assert_eq!(
            machine.cpu.state.a_long(i),
            0,
            "CPU address register A{i} must be cleared to 0 on cold reset"
        );
    }
    assert_eq!(
        machine.cpu.state.sr(),
        0x2700,
        "CPU Status Register must be reset to supervisor mask 7 ($2700)"
    );
    assert_eq!(
        machine.cpu.state.ipl, 0,
        "CPU IPL must be 0 with all interrupts masked/cleared"
    );
    assert!(
        !machine.cpu.state.reset_line_asserted,
        "CPU reset line asserted flag must be cleared"
    );
}

#[test]
fn test_warm_reset_full_flow() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);
    machine.physical_memory.map_chip_ram_to_low_memory();

    // 1. Pre-populate RAM buffers
    let _ = machine.memory_bus().write_byte(0x001000, 0x42);
    let _ = machine.memory_bus().write_byte(0x002000, 0x84);
    assert_eq!(machine.physical_memory.read_byte_debug(0x001000), 0x42);
    assert_eq!(machine.physical_memory.read_byte_debug(0x002000), 0x84);

    // 2. Modify custom chip registers
    machine.agnus.dma.write_dmacon(0x8200);
    machine.denise.color[0] = 0x00F0; // Green background
    machine.paula.write_intena(0x8080);

    // 3. Modify CPU registers
    machine.cpu.state.set_d_long(0, 0xCAFEBABE);
    machine.cpu.state.set_d_long(1, 0xDEADBEEF);
    machine.cpu.state.set_a_long(0, 0x00040000);
    machine.cpu.state.set_a_long(1, 0x00050000);

    // 4. Advance machine clocks
    machine.step_cycles(500);
    assert_eq!(machine.cck, 500);

    // 5. Trigger Warm Reset
    machine.reset_warm();

    // Verify clock counter reset
    assert_eq!(
        machine.cck, 0,
        "CCK counter must be reset to 0 on warm reset"
    );

    // Verify boot overlay re-engaged and Chip RAM preserved intact
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Boot overlay must be re-engaged on warm reset"
    );
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert_eq!(
        machine.physical_memory.read_byte_debug(0x001000),
        0x42,
        "Chip RAM contents must be preserved intact on warm reset"
    );
    assert_eq!(
        machine.physical_memory.read_byte_debug(0x002000),
        0x84,
        "Chip RAM contents must be preserved intact on warm reset"
    );

    // Verify custom chips reset to power-on defaults
    assert_eq!(
        machine.agnus.dma.dmacon, 0x0000,
        "DMACON must be reset to 0"
    );
    assert_eq!(
        machine.denise.color[0], 0x0000,
        "Palette must be reset to 0"
    );
    assert_eq!(
        machine.paula.interrupts.intena, 0x0000,
        "INTENA must be reset to 0"
    );

    // Verify CPU registers preserved intact
    assert_eq!(
        machine.cpu.state.d_long(0),
        0xCAFEBABE,
        "D0 must be preserved on warm reset"
    );
    assert_eq!(
        machine.cpu.state.d_long(1),
        0xDEADBEEF,
        "D1 must be preserved on warm reset"
    );
    assert_eq!(
        machine.cpu.state.a_long(0),
        0x00040000,
        "A0 must be preserved on warm reset"
    );
    assert_eq!(
        machine.cpu.state.a_long(1),
        0x00050000,
        "A1 must be preserved on warm reset"
    );
    assert_eq!(
        machine.cpu.state.sr(),
        0x2700,
        "CPU SR must be reset to supervisor mask 7 ($2700)"
    );
    assert_eq!(
        machine.cpu.state.ipl, 0,
        "CPU IPL must be 0 with all interrupts masked/cleared"
    );
}

#[test]
fn test_cpu_reset_instruction_external_propagation() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    // Populate Kickstart ROM with NOP ($4E71) and MOVEQ #42, D0 ($702A) at offset $1002..$1006
    // so execution can seamlessly continue when RESET re-engages the Gary boot overlay.
    let mut rom = vec![0xFF; 262144];
    rom[0x1000..0x1006].copy_from_slice(&[0x4E, 0x70, 0x4E, 0x71, 0x70, 0x2A]);
    machine.physical_memory.write_bytes_debug(0xF80000, &rom);

    // Disengage overlay to simulate post-boot state running from Chip RAM
    machine.physical_memory.map_chip_ram_to_low_memory();

    // Code at $001000:
    // $001000: RESET       ; 4E70 (Pushes external _RESET line for 124 clocks)
    // $001002: NOP         ; 4E71
    // $001004: MOVEQ #42, D0; 702A
    let code: [u16; 3] = [0x4E70, 0x4E71, 0x702A];
    for (idx, &word) in code.iter().enumerate() {
        let addr = 0x001000 + (idx as u32 * 2);
        let _ = machine.memory_bus().write_word(addr, word);
    }

    // Pre-populate RAM data at $003000
    let _ = machine.memory_bus().write_byte(0x003000, 0x77);
    assert_eq!(machine.physical_memory.read_byte_debug(0x003000), 0x77);

    // Modify custom chip registers
    machine.agnus.dma.write_dmacon(0x8200);
    machine.denise.color[0] = 0x0F00;
    machine.paula.write_intena(0x8080);

    // Modify CPU registers
    machine.cpu.state.set_d_long(0, 0x12345678);
    machine.cpu.state.set_a_long(0, 0x00050000);

    // Prime CPU execution at $001000 in supervisor mode
    machine
        .cpu
        .set_pc_and_prime_prefetch(0x001000, &mut machine.physical_memory);
    machine.cpu.state.set_sr(0x2700);

    // Step the RESET instruction through all sub-cycle microsteps
    machine.step_instruction();

    // 1. Verify custom chips WERE reset by the CPU RESET line pulse
    assert_eq!(
        machine.agnus.dma.dmacon, 0x0000,
        "DMACON must be reset by M68000 RESET instruction"
    );
    assert_eq!(
        machine.denise.color[0], 0x0000,
        "Palette must be reset by M68000 RESET instruction"
    );
    assert_eq!(
        machine.paula.interrupts.intena, 0x0000,
        "INTENA must be reset by M68000 RESET instruction"
    );

    // 2. Verify Gary boot overlay was re-engaged by RESET line pulse
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Gary boot overlay must be re-engaged by M68000 RESET instruction"
    );

    // 3. Verify Chip RAM was NOT touched
    assert_eq!(
        machine.physical_memory.chip_ram[0x003000], 0x77,
        "RAM must remain undisturbed by M68000 RESET instruction"
    );

    // 4. Verify CPU registers were NOT reset
    assert_eq!(
        machine.cpu.state.d_long(0),
        0x12345678,
        "CPU D0 must remain untouched by RESET instruction"
    );
    assert_eq!(
        machine.cpu.state.a_long(0),
        0x00050000,
        "CPU A0 must remain untouched by RESET instruction"
    );

    // 5. Verify CPU PC advanced past RESET to the next instruction ($001002 NOP)
    assert_eq!(
        machine.cpu.state.instruction_pc, 0x001002,
        "CPU PC must advance past RESET to the subsequent instruction"
    );
    assert!(
        !machine.cpu.state.reset_line_asserted,
        "Reset line assertion flag must be cleared after dispatch"
    );

    // 6. Verify CPU continues linear execution (fetching from re-engaged Kickstart ROM)
    machine.step_instruction(); // executes NOP
    assert_eq!(machine.cpu.state.instruction_pc, 0x001004);

    machine.step_instruction(); // executes MOVEQ #42, D0
    assert_eq!(machine.cpu.state.d_long(0), 42);
}

#[test]
fn test_cpu_reset_instruction_privilege_violation() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);
    machine.physical_memory.map_chip_ram_to_low_memory();

    // Code at $001000: RESET ($4E70)
    let _ = machine.memory_bus().write_word(0x001000, 0x4E70);

    // Privilege Violation vector (Vector 8 at $000020) points to $002600
    machine.physical_memory.write_word_debug(0x000020, 0x0000);
    machine.physical_memory.write_word_debug(0x000022, 0x002600);

    // Exception handler at $002600: MOVEQ #99, D0 ($7063), RTE ($4E73)
    let _ = machine.memory_bus().write_word(0x002600, 0x7063);
    let _ = machine.memory_bus().write_word(0x002602, 0x4E73);

    // Set custom chip register
    machine.agnus.dma.write_dmacon(0x8200);

    // Setup CPU in user mode (S = 0)
    machine.cpu.state.set_ssp(0x005000);
    machine.cpu.state.set_usp(0x003000);
    machine.cpu.state.set_a_long(7, 0x003000);
    machine.cpu.state.set_sr(0x0000); // User mode
    machine
        .cpu
        .set_pc_and_prime_prefetch(0x001000, &mut machine.physical_memory);

    // Step instruction: traps to Vector 8
    machine.step_instruction();

    // Verify PC entered exception handler at $002600
    assert_eq!(
        machine.cpu.state.instruction_pc, 0x002600,
        "CPU must trap to Privilege Violation handler at $002600"
    );

    // Verify custom chips WERE NOT reset
    assert_eq!(
        machine.agnus.dma.dmacon, 0x0200,
        "Custom chips must NOT be reset when RESET instruction is rejected for privilege violation"
    );
}

#[test]
fn test_keyboard_ctrl_amiga_amiga_warm_reset() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);
    machine.physical_memory.map_chip_ram_to_low_memory();

    // 1. Pre-populate RAM and custom chips
    let _ = machine.memory_bus().write_byte(0x001000, 0x99);
    machine.agnus.dma.write_dmacon(0x8200);
    machine.denise.color[0] = 0x000F; // Blue background

    // 2. Simulate hardware key presses for Ctrl + Left-Amiga + Right-Amiga
    machine.keyboard.key_down(keyboard::SCANCODE_CTRL);
    machine.keyboard.key_down(keyboard::SCANCODE_L_AMIGA);
    machine.keyboard.key_down(keyboard::SCANCODE_R_AMIGA);

    assert!(
        machine.keyboard.reset_line_asserted,
        "Keyboard must assert reset_line_asserted on Ctrl-Amiga-Amiga"
    );

    // 3. Step 1 Color Clock (machine loop samples keyboard reset line)
    machine.step_cck();

    // 4. Verify warm reset executed
    assert!(
        !machine.keyboard.reset_line_asserted,
        "Keyboard reset line asserted flag must be cleared after handling"
    );
    assert_eq!(machine.cck, 0, "Master CCK must be reset to 0");
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Keyboard warm reset must re-engage boot overlay"
    );
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert_eq!(
        machine.physical_memory.read_byte_debug(0x001000),
        0x99,
        "RAM contents must be preserved on keyboard warm reset"
    );
    assert_eq!(
        machine.agnus.dma.dmacon, 0x0000,
        "Custom chips must be reset on keyboard warm reset"
    );
    assert_eq!(
        machine.denise.color[0], 0x0000,
        "Palette must be reset on keyboard warm reset"
    );
    assert_eq!(
        machine.cpu.state.sr(),
        0x2700,
        "CPU SR must be restored to $2700"
    );
}

#[test]
fn test_reset_overlay_kickstart_vs_synthetic() {
    // Case 1: Unpopulated / default Kickstart ROM
    let mut machine = A500Machine::new(A500Config::default());
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Hardware reset must unconditionally engage low-memory boot overlay"
    );

    machine.reset();
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Cold reset must unconditionally engage boot overlay"
    );

    machine.reset_warm();
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Warm reset must unconditionally engage boot overlay"
    );

    // Case 2: Populated Kickstart ROM mode
    let mut rom = vec![0x00; 262144];
    // Initial SSP at $000000: $00080000
    rom[0..4].copy_from_slice(&0x00080000u32.to_be_bytes());
    // Initial PC at $000004: $00FC0002
    rom[4..8].copy_from_slice(&0x00FC0002u32.to_be_bytes());
    machine.physical_memory.write_bytes_debug(0xF80000, &rom);

    // Cold reset engages overlay and loads vectors
    machine.reset();
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Kickstart mode must engage low-memory boot overlay on cold reset"
    );
    assert_eq!(
        machine.cpu.state.ssp(),
        0x00080000,
        "SSP must be loaded from Kickstart ROM vector"
    );
    assert_eq!(
        machine.cpu.state.instruction_pc, 0x00FC0002,
        "PC must be loaded from Kickstart ROM vector"
    );

    // Disengage overlay via CIA-A Port A bit 0
    machine.cia_a.write_register(0x2, 0x01); // DDRA bit 0 output
    machine.cia_a.write_register(0x0, 0x01); // PRA bit 0 high (disengage _OVL)
    for _ in 0..10 {
        machine.step_cck();
    }
    assert!(
        !machine.physical_memory.is_low_memory_overlay_active(),
        "Overlay must disengage when CIA-A drives PRA bit 0 high"
    );

    // Warm reset re-engages overlay
    machine.reset_warm();
    assert!(
        machine.physical_memory.is_low_memory_overlay_active(),
        "Warm reset must re-engage boot overlay"
    );
}
