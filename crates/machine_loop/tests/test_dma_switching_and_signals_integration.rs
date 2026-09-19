#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! DMA Switching & Electronic Signal Propagation Machine Loop Integration Tests
//!
//! Tests DMACON bitwise SET/CLR semantics across all peer chips,
//! physical electronic propagation delays for in-flight mutations,
//! and unmapped bus floating read behavior.

mod common;
use common::MachineHarness;
use machine_loop::{AddressBus, BusResult};

#[test]
fn test_dmacon_bitwise_set_and_clear_across_subsystems() {
    let mut harness = MachineHarness::new();

    // 1. Initially all DMA channels are disabled
    assert_eq!(harness.machine.agnus.dmacon, 0);
    assert!(!harness.machine.agnus.copper.dma_enabled);
    assert!(!harness.machine.agnus.blitter.dma_enabled);
    assert!(!harness.machine.denise.sprites.dma_enabled);

    // 2. Set Master DMA (bit 9) + Copper (bit 7) + Blitter (bit 6) + Sprite (bit 5)
    // $8000 | $0200 | $0080 | $0040 | $0020 = $82E0
    harness.machine.write_custom_word(0x096, 0x82E0);
    harness.step_cck(2); // Physical propagation delay

    assert_eq!(harness.machine.agnus.dmacon & 0x02E0, 0x02E0);
    assert!(harness.machine.agnus.copper.dma_enabled);
    assert!(harness.machine.agnus.blitter.dma_enabled);
    assert!(harness.machine.denise.sprites.dma_enabled);

    // 3. Clear only Copper DMA: write $0080 (bit 15 is 0 -> clear bit 7)
    harness.machine.write_custom_word(0x096, 0x0080);
    harness.step_cck(2);

    assert_eq!(harness.machine.agnus.dmacon & 0x0080, 0);
    assert!(!harness.machine.agnus.copper.dma_enabled);
    // Other channels must remain enabled
    assert!(harness.machine.agnus.blitter.dma_enabled);
    assert!(harness.machine.denise.sprites.dma_enabled);

    // 4. Clear Master DMA (DMAEN bit 9): write $0200
    harness.machine.write_custom_word(0x096, 0x0200);
    harness.step_cck(2);

    // Master DMA disabled: all individual channel activities must cease
    assert_eq!(harness.machine.agnus.dmacon & 0x0200, 0);
    assert!(!harness.machine.agnus.is_dma_enabled(0x0040));
    assert!(!harness.machine.agnus.is_dma_enabled(0x0020));
    assert!(!harness.machine.agnus.blitter.dma_enabled);
    assert!(!harness.machine.denise.sprites.dma_enabled);
}

#[test]
fn test_register_write_electronic_propagation_delay() {
    let mut harness = MachineHarness::new();

    // Initial DIWSTRT is 0
    assert_eq!(harness.machine.denise.diwstrt, 0x0000);

    // Dispatch write to DIWSTRT ($DFF08E) = $2C81
    harness.machine.write_custom_word(0x08E, 0x2C81);

    // Cycle 0: Write has just been staged into the mutation pipeline, NOT yet committed
    assert_eq!(
        harness.machine.denise.diwstrt, 0x0000,
        "Register should not update instantaneously on cycle 0 before clock phase progression"
    );

    // Step 1 Color Clock: mutation matures and commits
    harness.machine.step_cck();

    // Cycle 1: DIWSTRT must now be committed
    assert_eq!(
        harness.machine.denise.diwstrt, 0x2C81,
        "Register must commit after propagation delay matures"
    );
}

#[test]
fn test_unmapped_custom_register_open_bus_read() {
    let mut harness = MachineHarness::new();

    // In Amiga hardware, reading unmapped custom chip registers returns open bus ($FFFF)
    let open_bus_val = harness.machine.memory_bus().read_word(0xDFF1FE);
    match open_bus_val {
        BusResult::Ready(val) => {
            assert_eq!(
                val, 0xFFFF,
                "Unmapped custom register read must return floating open bus $FFFF"
            );
        }
        BusResult::WaitState => panic!("Unexpected WaitState on unmapped custom register read"),
    }
}
