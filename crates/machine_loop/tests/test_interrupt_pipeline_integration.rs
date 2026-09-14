//! Interrupt Pipeline & Priority Arbitration Whole-Machine Integration Tests
//!
//! Verifies Paula INTENA/INTREQ priority encoding, master INTEN masking,
//! multi-interrupt escalation order (Levels 1..6), and CPU autovector exception handling.

mod common;
use common::MachineHarness;

#[test]
fn test_multiple_simultaneous_interrupts_priority_order() {
    let mut harness = MachineHarness::new();

    // Enable Level 1 (DSKBLK 0x0002), Level 3 (BLIT 0x0040), Level 4 (AUD0 0x0080), Level 6 (EXTER 0x2000)
    // plus master INTEN (bit 14 = 0x4000) with SET/CLR (bit 15 = 0x8000)
    harness
        .machine
        .dispatch_custom_write(0x09A, 0x8000 | 0x4000 | 0x2000 | 0x0080 | 0x0040 | 0x0002);

    // Request Level 1, Level 3, and Level 4 simultaneously in INTREQ
    harness
        .machine
        .dispatch_custom_write(0x09C, 0x8000 | 0x0080 | 0x0040 | 0x0002);

    // Wait 2 CCKs for write to commit through delay pipeline
    harness.step_cck(2);

    // Highest pending level should be Level 4 (Audio 0)
    assert_eq!(
        harness.machine.resolve_ipl(),
        4,
        "Level 4 should take precedence over Level 3 and Level 1"
    );

    // Clear Level 4 in INTREQ (bit 15 = 0 to clear, bit 7 = AUD0)
    harness.machine.dispatch_custom_write(0x09C, 0x0080);
    harness.step_cck(2);

    // Next pending level should be Level 3 (Blitter)
    assert_eq!(
        harness.machine.resolve_ipl(),
        3,
        "Level 3 should take precedence after Level 4 is cleared"
    );

    // Clear Level 3 in INTREQ
    harness.machine.dispatch_custom_write(0x09C, 0x0040);
    harness.step_cck(2);

    // Next pending level should be Level 1 (Disk block)
    assert_eq!(
        harness.machine.resolve_ipl(),
        1,
        "Level 1 should be active after Level 3 is cleared"
    );

    // Clear Level 1 in INTREQ
    harness.machine.dispatch_custom_write(0x09C, 0x0002);
    harness.step_cck(2);

    assert_eq!(
        harness.machine.resolve_ipl(),
        0,
        "IPL should return to 0 when all requests are cleared"
    );
}

#[test]
fn test_intena_master_and_individual_channel_masking() {
    let mut harness = MachineHarness::new();

    // Request Level 4 (AUD0 = 0x0080) and Level 3 (BLIT = 0x0040)
    harness
        .machine
        .dispatch_custom_write(0x09C, 0x8000 | 0x0080 | 0x0040);
    harness.step_cck(2);

    // With INTENA = 0 (or master INTEN disabled), no IPL is asserted
    assert_eq!(
        harness.machine.resolve_ipl(),
        0,
        "Interrupts must be suppressed when INTENA is not enabled"
    );

    // Enable only Level 3 and master INTEN in INTENA (0xC040)
    harness
        .machine
        .dispatch_custom_write(0x09A, 0x8000 | 0x4000 | 0x0040);
    harness.step_cck(2);

    // Although Level 4 is in INTREQ, it is masked in INTENA, so Level 3 should be resolved
    assert_eq!(
        harness.machine.resolve_ipl(),
        3,
        "Only unmasked Level 3 should be resolved"
    );

    // Clear master INTEN (bit 15 = 0, bit 14 = 1 in INTENA write)
    harness.machine.dispatch_custom_write(0x09A, 0x4000);
    harness.step_cck(2);

    assert_eq!(
        harness.machine.resolve_ipl(),
        0,
        "Clearing master INTEN should immediately suppress all custom interrupts"
    );

    // Re-enable master INTEN
    harness.machine.dispatch_custom_write(0x09A, 0xC000);
    harness.step_cck(2);

    assert_eq!(
        harness.machine.resolve_ipl(),
        3,
        "Re-enabling master INTEN should immediately restore Level 3"
    );
}

#[test]
fn test_cpu_autovector_exception_dispatch() {
    let mut harness = MachineHarness::new();

    let isr_level3_addr = 0x001000;
    let main_code_addr = 0x002000;

    // Set Level 3 Autovector (Vector 27 = address 27 * 4 = $6C)
    harness.set_vector(27, isr_level3_addr);

    // Load simple infinite loop in main code: BRA.S * ($60FE)
    harness.load_cpu_code(main_code_addr, &[0x60FE]);

    // ISR code at $001000: NOP ($4E71), RTE ($4E73)
    harness.load_cpu_code(isr_level3_addr, &[0x4E71, 0x4E73]);
    harness.machine.set_pc_and_prime_prefetch(main_code_addr);

    // Enable Level 3 and master INTEN in INTENA
    harness.machine.dispatch_custom_write(0x09A, 0xC040);
    // Request Level 3 in INTREQ
    harness.machine.dispatch_custom_write(0x09C, 0x8040);
    harness.step_cck(2);

    assert_eq!(harness.machine.resolve_ipl(), 3);

    // Step CPU instruction: finishes current instruction, takes interrupt on boundary
    harness.machine.step_instruction();
    harness.machine.step_instruction();

    // Verify PC entered the Level 3 ISR
    assert_eq!(
        harness.machine.cpu.state.instruction_pc, isr_level3_addr,
        "CPU PC should vector to Level 3 ISR address ($001000)"
    );

    // Verify CPU SR interrupt mask was raised to at least 3
    let mask = (harness.machine.cpu.state.sr >> 8) & 0x07;
    assert!(
        mask >= 3,
        "CPU interrupt mask in SR should be raised to >= 3"
    );
}
