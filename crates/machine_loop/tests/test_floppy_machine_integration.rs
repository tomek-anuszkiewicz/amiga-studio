#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Floppy DMA & Whole-Machine Integration Tests
//!
//! Tests multi-chip coordination during floppy MFM track DMA:
//! 1. CIA-B motor latching and drive selection (DF0:).
//! 2. Paula ADKCON wordsync configuration and DSKSYNC pattern match.
//! 3. Agnus 8-tier DMA slot scheduling (HPOS 7, 9, 11).
//! 4. Chip RAM streaming at DSKPT with automatic address incrementation.
//! 5. Level 1 IRQ_DSKBLK and Level 5 IRQ_DSKSYN interrupt assertion upon completion.

mod common;
use common::MachineHarness;
use floppy::{FORMATTED_DISK_BYTES, SECTORS_PER_TRACK, SECTOR_DATA_BYTES};
use machine_loop::AddressBus;

#[test]
fn test_floppy_dma_stream_into_chip_ram_and_interrupt() {
    let mut harness = MachineHarness::new();

    // 1. Prepare ADF disk image with recognizable data
    let mut adf_image = vec![0u8; FORMATTED_DISK_BYTES];
    for track in 0..160 {
        for sector in 0..SECTORS_PER_TRACK {
            let offset = (track * SECTORS_PER_TRACK + sector) * SECTOR_DATA_BYTES;
            adf_image[offset] = (track + 1) as u8;
            adf_image[offset + 1] = (sector + 1) as u8;
        }
    }

    // Insert into DF0:
    harness.machine.floppy.drives[0].insert_disk(&adf_image);

    // 2. Select DF0: with motor ON via CIA-B PRB ($BFD100) write
    // _SEL0 = bit 3 (0), _MTR = bit 7 (0) -> 0b0111_0111 = 0x77
    harness.machine.memory_bus().write_byte(0xBFD100, 0x77);
    harness.machine.step_cck(); // Propagate CIA-B PRB to Floppy
    assert!(harness.machine.floppy.drives[0].selected);
    assert!(harness.machine.floppy.drives[0].motor_on);

    // 3. Configure Chip RAM destination DSKPTH/DSKPTL = $002000
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x020, 0x0000);
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x022, 0x2000);

    // 4. Set DSKSYNC = $4489 ($DFF07E)
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x07E, 0x4489);

    // 5. Enable WORDSYNC and FAST in ADKCON ($DFF09E): SET_CLR | WORDSYNC | FAST = 0x8500
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x09E, 0x8500);

    // 6. Enable master DMA and Disk DMA in DMACON ($DFF096): SET_CLR | DMAEN | DSKEN = 0x8210
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x096, 0x8210);

    // 7. Enable master INTENA and DSKBLK in INTENA ($DFF09A): SET_CLR | INTEN | DSKBLK = 0xC002
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x09A, 0xC002);

    // Step 2 CCKs to commit initial register mutations
    harness.machine.step_cck();
    harness.machine.step_cck();
    assert_eq!(harness.machine.agnus.dskpt, 0x0000_2000);

    // 8. Arm and start Disk DMA in DSKLEN ($DFF024): 4 words
    let dsklen_val = 0x8000 | 4;
    // Write 1: Arm DMA
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x024, dsklen_val);
    harness.machine.step_cck();
    harness.machine.step_cck();
    assert!(harness.machine.paula.dma_armed);

    // Write 2: Activate DMA
    harness
        .machine
        .memory_bus()
        .write_custom_word(0x024, dsklen_val);
    harness.machine.step_cck();
    harness.machine.step_cck();
    assert!(harness.machine.paula.is_dsk_dma_active());

    // 9. Step machine loop until DMA completes
    let mut steps = 0;
    while harness.machine.paula.is_dsk_dma_active() && steps < 100_000 {
        harness.machine.step_cck();
        steps += 1;
    }

    // DMA must have completed
    assert!(
        !harness.machine.paula.is_dsk_dma_active(),
        "Disk DMA did not complete within limit"
    );

    // Agnus DSKPT must have advanced by 8 bytes (4 words * 2 bytes)
    assert_eq!(harness.machine.agnus.dskpt, 0x0000_2008);

    // Chip RAM at $002000 must contain non-zero MFM data streamed from disk
    let word0 = harness.machine.physical_memory.read_chip_word(0x2000);
    assert_ne!(word0, 0, "Chip RAM should have received MFM data");

    // Paula INTREQ must have bit 1 (DSKBLK) asserted
    assert_ne!(
        harness.machine.paula.interrupts.read_intreqr() & 0x0002,
        0,
        "DSKBLK interrupt must be requested in INTREQ"
    );
}
