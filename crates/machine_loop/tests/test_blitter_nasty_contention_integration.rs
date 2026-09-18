//! Blitter Nasty Contention & Fast RAM Concurrency Whole-Machine Integration Tests
//!
//! Verifies that active Blitter Nasty mode (BLTPRI) locks CPU accesses out of Chip RAM
//! with WaitState responses while Fast RAM ($200000) execution continues with 100% immunity.

mod common;
use common::MachineHarness;
use config::A500Preset;
use physical_memory::BusResult;

#[test]
fn test_blitter_nasty_blocks_chip_ram_and_allows_fast_ram() {
    let mut harness = MachineHarness::new_with_preset(A500Preset::ExpandedPowerUser);

    // Initialize test data in Chip RAM ($001000) and Fast RAM ($200000)
    harness
        .machine
        .physical_memory
        .write_word_debug(0x001000, 0x1111);
    harness
        .machine
        .physical_memory
        .write_word_debug(0x200000, 0x2222);

    // Setup Blitter for a 20-word transfer in Chip RAM
    // BLTCON0: Minterm 0xCA (D = A), use channel A and D (0x09CA)
    harness.machine.write_custom_word(0x040, 0x09CA);
    harness.machine.write_custom_word(0x042, 0x0000); // BLTCON1
    harness.machine.write_custom_word(0x050, 0x001000); // BLTAPTH/L
    harness.machine.write_custom_word(0x054, 0x002000); // BLTDPTH/L
    harness.machine.write_custom_word(0x064, 0); // BLTAMOD
    harness.machine.write_custom_word(0x066, 0); // BLTDMOD

    // Enable Blitter Nasty in DMACON: SET | DMAEN | BLTEN | BLTPRI -> 0x8640
    harness
        .machine
        .write_custom_word(0x096, 0x8000 | 0x0200 | 0x0040 | 0x0400);

    // Start Blitter: 10 lines x 2 words = 20 words
    harness.machine.write_custom_word(0x058, (10 << 6) | 2);

    // Step 2 CCKs for registers to commit and Blitter to engage
    harness.step_cck(2);

    // Assert Blitter is running
    assert!(
        harness.machine.agnus.blitter.is_busy,
        "Blitter should be busy executing transfer"
    );

    // Step machine 1 CCK so Agnus DMA arbiter evaluates Blitter Nasty bus lock
    harness.machine.step_subsystems_cck();

    // Verify Chip RAM is reported blocked by Agnus
    assert!(
        harness.machine.agnus.chip_ram_blocked,
        "Agnus must assert chip_ram_blocked during Blitter Nasty"
    );
    assert!(
        harness.machine.physical_memory.chip_ram_blocked,
        "PhysicalMemory must mirror chip_ram_blocked"
    );

    // 1. Chip RAM access must stall with WaitState
    let chip_read = harness.machine.physical_memory.read_word(0x001000);
    assert_eq!(
        chip_read,
        BusResult::WaitState,
        "CPU access to Chip RAM must return WaitState during Blitter Nasty"
    );

    // 2. Fast RAM access must succeed immediately with BusResult::Ready
    let fast_read = harness.machine.physical_memory.read_word(0x200000);
    assert_eq!(
        fast_read,
        BusResult::Ready(0x2222),
        "Fast RAM must remain completely immune to Chip RAM Blitter Nasty contention"
    );

    // Step until Blitter completes transfer
    harness.step_cck(150);

    assert!(
        !harness.machine.agnus.blitter.is_busy,
        "Blitter should finish transfer"
    );
    assert!(
        !harness.machine.agnus.chip_ram_blocked,
        "Agnus must release chip_ram_blocked once Blitter finishes"
    );

    // Chip RAM access must now succeed
    let chip_read_after = harness.machine.physical_memory.read_word(0x001000);
    assert_eq!(
        chip_read_after,
        BusResult::Ready(0x1111),
        "Chip RAM access must resume normally after Blitter finishes"
    );
}

#[test]
fn test_blitter_nasty_fast_ram_cpu_concurrency() {
    let mut harness = MachineHarness::new_with_preset(A500Preset::ExpandedPowerUser);

    let fast_code_addr = 0x200000;

    // Fast RAM program:
    // ADDQ.L #1, D0 ($5280)
    // BRA.S *-2     ($60FE)
    let fast_code = [0x5280, 0x60FE];
    for (i, &w) in fast_code.iter().enumerate() {
        harness
            .machine
            .physical_memory
            .write_word_debug(fast_code_addr + (i as u32) * 2, w);
    }
    harness.machine.set_pc_and_prime_prefetch(fast_code_addr);
    harness.machine.cpu.state.set_d_long(0, 0);

    // Setup a long Blitter transfer in Chip RAM: 200 lines x 2 words = 400 words
    harness.machine.write_custom_word(0x040, 0x09CA);
    harness.machine.write_custom_word(0x050, 0x001000);
    harness.machine.write_custom_word(0x054, 0x002000);
    harness.machine.write_custom_word(0x096, 0x8640); // SET + DMAEN + BLTEN + BLTPRI
    harness.machine.write_custom_word(0x058, (200 << 6) | 2);
    harness.step_cck(2);

    assert!(harness.machine.agnus.blitter.is_busy);

    // Step machine across 50 CCKs while Blitter Nasty is running
    harness.step_cck(50);

    // Verify Blitter is still busy
    assert!(harness.machine.agnus.blitter.is_busy);

    // Verify CPU executing in Fast RAM made continuous progress without stalling!
    let d0 = harness.machine.cpu.state.d_long(0);
    assert!(
        d0 > 0,
        "CPU running in Fast RAM should increment D0 even while Blitter Nasty is active in Chip RAM"
    );
}
