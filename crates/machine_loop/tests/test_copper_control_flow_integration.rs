//! Copper Advanced Control Flow Whole-Machine Integration Tests
//!
//! Verifies Copper SKIP instruction condition evaluation (skip taken vs fallthrough)
//! and Copper Danger mode (CDANG in COPCON) register write protection in the machine loop.

mod common;
use common::MachineHarness;

#[test]
fn test_copper_skip_taken_when_beam_past_target() {
    let mut harness = MachineHarness::new();
    let copper_list_addr = 0x001000;

    // Copper list:
    // 1. SKIP when VPOS >= 5, HPOS >= 0 (Word 1: 0x0501, Word 2: 0xFFFF)
    // 2. Skipped instruction: MOVE #$0F00, COLOR00 (0x0180, 0x0F00 - Red)
    // 3. Executed instruction: MOVE #$00F0, COLOR00 (0x0180, 0x00F0 - Green)
    // 4. End with WAIT ($FFFF, $FFFE)
    let copper_words = [
        0x0501, 0xFFFF, // SKIP(vpos=5, hpos=0)
        0x0180, 0x0F00, // MOVE #$0F00, COLOR00 (Red)
        0x0180, 0x00F0, // MOVE #$00F0, COLOR00 (Green)
        0xFFFF, 0xFFFE, // WAIT end of frame
    ];

    harness.load_copper_list(copper_list_addr, &copper_words);

    // Enable Copper DMA in DMACON (0x8280 = SET + DMAEN + COPEN)
    harness.machine.dispatch_custom_write(0x096, 0x8280);

    // Advance beam to line 6 (past line 5)
    harness.step_scanlines(6);

    // Jump to list 1 to start executing at current beam position
    harness.machine.dispatch_custom_write(0x088, 0x0000); // COPJMP1

    // Step a few CCKs to execute SKIP and the succeeding MOVE
    harness.step_cck(20);

    // Because beam is at line 6 (>= 5), the Red write (0x0F00) must have been SKIPPED,
    // and COLOR00 must be Green (0x00F0)
    assert_eq!(
        harness.machine.denise.color[0], 0x00F0,
        "Copper SKIP should have bypassed the Red write and committed Green"
    );
}

#[test]
fn test_copper_skip_not_taken_when_beam_before_target() {
    let mut harness = MachineHarness::new();
    let copper_list_addr = 0x001000;

    // Copper list:
    // 1. SKIP when VPOS >= 50 (Word 1: 0x3201, Word 2: 0xFFFF)
    // 2. Next instruction: MOVE #$0F00, COLOR00 (0x0180, 0x0F00 - Red)
    // 3. WAIT end of frame ($FFFF, $FFFE)
    let copper_words = [
        0x3201, 0xFFFF, // SKIP(vpos=50, hpos=0)
        0x0180, 0x0F00, // MOVE #$0F00, COLOR00 (Red)
        0xFFFF, 0xFFFE, // WAIT end of frame
    ];

    harness.load_copper_list(copper_list_addr, &copper_words);
    harness.machine.dispatch_custom_write(0x096, 0x8280);

    // Beam starts at line 0 (well before line 50)
    harness.machine.dispatch_custom_write(0x088, 0x0000); // COPJMP1
    harness.step_cck(20);

    // Because beam is at line 0 (< 50), the Red write must NOT be skipped
    assert_eq!(
        harness.machine.denise.color[0], 0x0F00,
        "Copper SKIP should not trigger when beam is before target position"
    );
}

#[test]
fn test_copper_cdang_danger_mode_protection() {
    let mut harness = MachineHarness::new();
    let copper_list_addr = 0x001000;

    // Copper list attempting to write to BLTCON0 ($040, below $080 threshold):
    // 1. MOVE #$1234, BLTCON0 ($0040, 0x1234)
    // 2. WAIT end of frame
    let copper_words = [
        0x0040, 0x1234, // MOVE #$1234, BLTCON0
        0xFFFF, 0xFFFE,
    ];

    harness.load_copper_list(copper_list_addr, &copper_words);
    harness.machine.dispatch_custom_write(0x096, 0x8280);

    // Case 1: CDANG is 0 (default COPCON = 0)
    assert!(!harness.machine.agnus.copper.cdang);
    harness.machine.dispatch_custom_write(0x088, 0x0000); // COPJMP1
    harness.step_cck(20);

    assert_eq!(
        harness.machine.agnus.blitter.bltcon0, 0,
        "Copper write to register < $080 must be blocked when CDANG is 0"
    );

    // Case 2: Enable CDANG in COPCON ($02E) by setting bit 1
    harness.machine.dispatch_custom_write(0x02E, 0x0002);
    harness.step_cck(2);
    assert!(harness.machine.agnus.copper.cdang);

    // Restart list with CDANG active
    harness.machine.dispatch_custom_write(0x088, 0x0000); // COPJMP1
    harness.step_cck(20);

    assert_eq!(
        harness.machine.agnus.blitter.bltcon0, 0x1234,
        "Copper write to register < $080 must succeed when CDANG is enabled"
    );
}
