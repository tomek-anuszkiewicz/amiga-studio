//! Blitter & Machine Loop Integration Tests
//!
//! Tests Blitter 2D memory operations (copy, fill), Chip RAM mutation,
//! DMA scheduling, and cross-chip completion signaling (_BLITINT) to Paula.

mod common;
use common::MachineHarness;

#[test]
fn test_blitter_copy_and_interrupt_propagation() {
    let mut harness = MachineHarness::new();

    let src_addr: u32 = 0x010000;
    let dst_addr: u32 = 0x010100;

    // 1. Prepare test pattern in Chip RAM source buffer
    let mut pattern = [0u16; 16];
    for (i, item) in pattern.iter_mut().enumerate() {
        *item = 0x0A00 | (i as u16 + 1);
    }
    for (i, &w) in pattern.iter().enumerate() {
        harness
            .machine
            .physical_memory
            .write_word_debug(src_addr + (i as u32) * 2, w);
        // Ensure destination is zeroed
        harness
            .machine
            .physical_memory
            .write_word_debug(dst_addr + (i as u32) * 2, 0x0000);
    }

    // 2. Enable Master DMA + Blitter DMA in DMACON ($DFF096): $8240
    harness.machine.dispatch_custom_write(0x096, 0x8240);
    // Enable Paula Blitter IRQ in INTENA ($DFF09A): Master + Blit IRQ bit 6 -> $C040
    harness.machine.dispatch_custom_write(0x09A, 0xC040);
    harness.step_cck(2); // Commit pipeline mutations

    // 3. Program Blitter registers:
    // BLTCON0 ($040): USEA (bit 11), USED (bit 8), minterm $F0 (copy A to D) -> $09F0
    harness.machine.dispatch_custom_write(0x040, 0x09F0);
    // BLTCON1 ($042): 0
    harness.machine.dispatch_custom_write(0x042, 0x0000);
    // BLTAFWM / BLTALWM ($044 / $046): $FFFF
    harness.machine.dispatch_custom_write(0x044, 0xFFFF);
    harness.machine.dispatch_custom_write(0x046, 0xFFFF);
    // Modulos ($064, $066): 0
    harness.machine.dispatch_custom_write(0x064, 0x0000);
    harness.machine.dispatch_custom_write(0x066, 0x0000);
    // BLTAPTH / BLTAPTL ($050 / $052)
    harness
        .machine
        .dispatch_custom_write(0x050, (src_addr >> 16) as u16);
    harness
        .machine
        .dispatch_custom_write(0x052, (src_addr & 0xFFFF) as u16);
    // BLTDPTH / BLTDPTL ($054 / $056)
    harness
        .machine
        .dispatch_custom_write(0x054, (dst_addr >> 16) as u16);
    harness
        .machine
        .dispatch_custom_write(0x056, (dst_addr & 0xFFFF) as u16);

    harness.step_cck(2); // Commit setup

    // 4. Start Blit by writing BLTSIZE ($058): 4 rows x 4 words = 16 words
    // HSIZE in bits 5..0 (4), VSIZE in bits 15..6 (4) -> (4 << 6) | 4 = $0104
    harness.machine.dispatch_custom_write(0x058, 0x0104);
    harness.step_cck(2);

    // Assert Blitter is running
    assert!(harness.machine.agnus.blitter.is_busy);

    // 5. Step machine loop until Blitter finishes
    let mut cck_count = 0;
    while harness.machine.agnus.blitter.is_busy {
        harness.machine.step_cck();
        cck_count += 1;
        assert!(
            cck_count < 1000,
            "Timed out waiting for Blitter to finish 16-word copy"
        );
    }

    // 6. Assert destination buffer in Chip RAM was modified to match source
    for (i, &expected_word) in pattern.iter().enumerate() {
        let actual = harness
            .machine
            .physical_memory
            .read_word_debug(dst_addr + (i as u32) * 2);
        assert_eq!(
            actual, expected_word,
            "Blitter copy mismatch at word offset {}",
            i
        );
    }

    // 7. Step 2 CCKs for Agnus blitter IRQ to latch into Paula INTREQ
    harness.step_cck(2);

    // 8. Assert Paula INTREQ bit 6 is set and CPU IPL resolves to Level 3
    assert_eq!(
        harness.machine.paula.intreq & 0x0040,
        0x0040,
        "Paula INTREQ bit 6 (_BLITINT) was not asserted upon Blitter completion"
    );
    assert_eq!(
        harness.machine.resolve_ipl(),
        3,
        "CPU IPL should be 3 following Blitter completion"
    );
}

#[test]
fn test_blitter_fill_operation_mutates_ram() {
    let mut harness = MachineHarness::new();

    let dst_addr: u32 = 0x020000;
    let word_count = 8;

    // Zero out destination RAM
    for i in 0..word_count {
        harness
            .machine
            .physical_memory
            .write_word_debug(dst_addr + i * 2, 0x0000);
    }

    // Enable Master DMA + Blitter DMA
    harness.machine.dispatch_custom_write(0x096, 0x8240);
    harness.step_cck(2);

    // Program Blitter to Fill with 1s (USED only, LF = $FF)
    // BLTCON0: USED (bit 8), LF = $FF -> $01FF
    harness.machine.dispatch_custom_write(0x040, 0x01FF);
    harness.machine.dispatch_custom_write(0x042, 0x0000);
    harness.machine.dispatch_custom_write(0x066, 0x0000); // D modulo = 0
    harness
        .machine
        .dispatch_custom_write(0x054, (dst_addr >> 16) as u16);
    harness
        .machine
        .dispatch_custom_write(0x056, (dst_addr & 0xFFFF) as u16);
    harness.step_cck(2);

    // Start Blit: 2 rows of 4 words
    harness.machine.dispatch_custom_write(0x058, 0x0084);
    harness.step_cck(2);

    // Step until complete
    let mut cck = 0;
    while harness.machine.agnus.blitter.is_busy {
        harness.machine.step_cck();
        cck += 1;
        assert!(cck < 500, "Blitter fill timed out");
    }

    // Verify all 8 words were filled with $FFFF
    for i in 0..word_count {
        let actual = harness
            .machine
            .physical_memory
            .read_word_debug(dst_addr + i * 2);
        assert_eq!(actual, 0xFFFF, "Blitter fill mismatch at word {}", i);
    }
}
