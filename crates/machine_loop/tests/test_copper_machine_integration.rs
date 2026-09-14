//! Copper & Machine Loop Integration Tests
//!
//! Tests Copper coprocessor list execution, beam position synchronization (WAIT),
//! custom register mutations (MOVE into Denise COLOR00), and DMA enabling/disabling
//! in the coordinated A500Machine loop.

mod common;
use common::MachineHarness;

#[test]
fn test_copper_beam_wait_and_palette_mutation() {
    let mut harness = MachineHarness::new();

    // Copper list at $002000:
    // 1. WAIT for scanline 15: $0F01, $FFFE (V=15, H=0)
    // 2. MOVE $0F00 -> COLOR00 ($DFF180) (Red)
    // 3. WAIT for scanline 30: $1E01, $FFFE (V=30, H=0)
    // 4. MOVE $00F0 -> COLOR00 ($DFF180) (Green)
    // 5. End of list: $FFFF, $FFFE
    let copper_list = [
        0x0F01, 0xFFFE, // WAIT (vpos: 15, hpos: 0)
        0x0180, 0x0F00, // MOVE #$0F00, COLOR00
        0x1E01, 0xFFFE, // WAIT (vpos: 30, hpos: 0)
        0x0180, 0x00F0, // MOVE #$00F0, COLOR00
        0xFFFF, 0xFFFE, // End of Copper list
    ];

    harness.load_copper_list(0x002000, &copper_list);

    // Initial palette check: COLOR00 is 0
    assert_eq!(harness.machine.denise.color[0], 0x0000);

    // Enable DMA: Master Enable (bit 9) | Copper Enable (bit 7) -> $8280
    harness.machine.dispatch_custom_write(0x096, 0x8280);
    // Strobe COPJMP1 ($088) to start execution of list 1
    harness.machine.dispatch_custom_write(0x088, 0x0000);

    // Step across first 10 scanlines: beam hasn't reached line 15 yet
    harness.step_scanlines(10);
    assert_eq!(
        harness.machine.denise.color[0], 0x0000,
        "Color register should not change before target scanline"
    );

    // Step past line 15 (up to line 20)
    harness.step_until_vpos(20, 5000);
    assert_eq!(
        harness.machine.denise.color[0], 0x0F00,
        "Copper failed to execute MOVE #$0F00, COLOR00 on scanline 15"
    );

    // Step past line 30 (up to line 35)
    harness.step_until_vpos(35, 5000);
    assert_eq!(
        harness.machine.denise.color[0], 0x00F0,
        "Copper failed to execute MOVE #$00F0, COLOR00 on scanline 30"
    );
}

#[test]
fn test_copper_dma_toggle_stops_and_resumes_execution() {
    let mut harness = MachineHarness::new();

    // Copper list: WAIT line 10 -> MOVE Red -> WAIT line 20 -> MOVE Green -> END
    let copper_list = [
        0x0A01, 0xFFFE, // WAIT line 10
        0x0180, 0x0F00, // MOVE #$0F00, COLOR00
        0x1401, 0xFFFE, // WAIT line 20
        0x0180, 0x00F0, // MOVE #$00F0, COLOR00
        0xFFFF, 0xFFFE,
    ];

    harness.load_copper_list(0x003000, &copper_list);

    // Start with Copper DMA enabled
    harness.machine.dispatch_custom_write(0x096, 0x8280);
    harness.machine.dispatch_custom_write(0x088, 0x0000);

    // Step to line 5, then DISABLE Copper DMA ($0080 clears bit 7)
    harness.step_until_vpos(5, 2000);
    harness.machine.dispatch_custom_write(0x096, 0x0080);
    harness.step_cck(2);
    assert!(!harness.machine.agnus.copper.dma_enabled);

    // Step past line 15 with DMA disabled
    harness.step_until_vpos(15, 3000);
    assert_eq!(
        harness.machine.denise.color[0], 0x0000,
        "Copper executed while DMA was disabled"
    );

    // Re-enable Copper DMA ($8080 sets bit 7) and restart list
    harness.machine.dispatch_custom_write(0x096, 0x8080);
    harness.machine.dispatch_custom_write(0x088, 0x0000);
    harness.step_cck(2);
    assert!(harness.machine.agnus.copper.dma_enabled);

    // Now step past line 25, verify color executed
    harness.step_until_vpos(25, 4000);
    assert_eq!(
        harness.machine.denise.color[0], 0x00F0,
        "Copper should complete instructions after DMA re-enabled"
    );
}

#[test]
fn test_copper_triggers_interrupt_to_cpu() {
    let mut harness = MachineHarness::new();

    // Setup Copper list writing to INTREQ ($09C) = $8010 (Set bit 15, Copper IRQ bit 4)
    let copper_list = [
        0x0501, 0xFFFE, // WAIT line 5
        0x009C, 0x8010, // MOVE #$8010, INTREQ (Assert Level 3 Copper interrupt)
        0xFFFF, 0xFFFE,
    ];

    harness.load_copper_list(0x004000, &copper_list);

    // Enable Paula Level 3 Interrupts: INTENA ($09A) = $C010 (Master + Copper)
    harness.machine.dispatch_custom_write(0x09A, 0xC010);
    // Enable Copper DMA
    harness.machine.dispatch_custom_write(0x096, 0x8280);
    harness.machine.dispatch_custom_write(0x088, 0x0000);

    // Initial interrupt priority line should be 0
    assert_eq!(harness.machine.resolve_ipl(), 0);

    // Step until line 8
    harness.step_until_vpos(8, 3000);

    // Verify Copper wrote to INTREQ and Paula raised Level 3 IPL
    assert_eq!(harness.machine.paula.intreq & 0x0010, 0x0010);
    assert_eq!(
        harness.machine.resolve_ipl(),
        3,
        "CPU IPL should be 3 following Copper interrupt"
    );
}
