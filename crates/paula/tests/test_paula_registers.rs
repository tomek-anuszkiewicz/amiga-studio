use paula::Paula;

#[test]
fn test_intena_intreq_propagation_delay_1_cck() {
    let mut paula = Paula::new();

    // Write INTENA at cycle T (SET INTEN master + Level 3 VBlank)
    paula.write_register(0x09A, 0xC020);

    // At cycle T: not yet committed
    assert_eq!(paula.interrupts.intena, 0);
    assert_eq!(paula.read_register(0x01C), 0);

    // Step 1 CCK (Cycle T+1): matures and commits!
    paula.step_cck();
    assert_eq!(paula.interrupts.intena, 0x4020);
    assert_eq!(paula.read_register(0x01C), 0x4020);

    // Write INTREQ at cycle T+1 (SET Level 3 VBlank request)
    paula.write_register(0x09C, 0x8020);
    assert_eq!(paula.interrupts.intreq, 0);
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Step 1 CCK (Cycle T+2): matures and commits, triggering IPL 3!
    paula.step_cck();
    assert_eq!(paula.interrupts.intreq, 0x0020);
    assert_eq!(paula.pending_interrupt_level(), 3);
}

#[test]
fn test_adkcon_set_clr_and_delay_2_cck() {
    let mut paula = Paula::new();

    // Write ADKCON: SET PRECOMP1 + WORDSYNC (bits 14 + 10 = 0x8000 | 0x4400)
    paula.write_register(0x09E, 0xC400);

    // 1 CCK passes: still 0
    paula.step_cck();
    assert_eq!(paula.adkcon, 0);

    // 2nd CCK passes: commits!
    paula.step_cck();
    assert_eq!(paula.adkcon, 0x4400);
    assert_eq!(paula.read_register(0x010), 0x4400);

    // Clear WORDSYNC (bit 10 = 0x0400, bit 15 = 0)
    paula.write_register(0x09E, 0x0400);
    paula.step_cck();
    assert_eq!(paula.adkcon, 0x4400); // still in flight
    paula.step_cck();
    assert_eq!(paula.adkcon, 0x4000); // WORDSYNC cleared, PRECOMP1 remains set
}

#[test]
fn test_dmacon_broadcast_to_paula() {
    let mut paula = Paula::new();

    // Write DMACON broadcast: SET DSKEN (bit 4) + AUD0EN (bit 0)
    paula.write_register(0x096, 0x8011);

    // 1 CCK: in flight
    paula.step_cck();
    assert_eq!(paula.dma_enables, 0);

    // 2 CCK: commits
    paula.step_cck();
    assert_eq!(paula.dma_enables, 0x0011);
}

#[test]
fn test_write_only_registers_read_open_bus() {
    let paula = Paula::new();
    assert_eq!(paula.read_register(0x09A), 0xFFFF);
    assert_eq!(paula.read_register(0x09C), 0xFFFF);
    assert_eq!(paula.read_register(0x0A4), 0xFFFF);
}

#[test]
fn test_paula_assemble_dskbytr() {
    let mut paula = Paula::new();
    let floppy_word = 0x80A5;

    // 1. Without DMA and without write mode
    assert_eq!(paula.assemble_dskbytr(floppy_word), 0x80A5);

    // 2. Enable DMA: master + DSKEN
    paula.dma_master = true;
    paula.dma_enables = paula::DSKBYTR_DSKEN;
    assert_eq!(
        paula.assemble_dskbytr(floppy_word),
        0x80A5 | paula::DSKBYTR_DMAON
    );

    // 3. Enable write mode in DSKLEN
    paula.dsklen = paula::DSKLEN_WRITE_FLAG;
    assert_eq!(
        paula.assemble_dskbytr(floppy_word),
        0x80A5 | paula::DSKBYTR_DMAON | paula::DSKBYTR_DISKWRITE
    );
}

#[test]
fn test_paula_register_getters() {
    let mut paula = Paula::new();
    paula.adkcon = 0x1234;
    paula.pot0dat = 0x2345;
    paula.pot1dat = 0x3456;
    paula.potgor = 0x4567;
    paula.serial_port.serdatr = 0x5678;
    paula.interrupts.intena = 0x6789;
    paula.interrupts.intreq = 0x789A;

    assert_eq!(paula.adkconr(), 0x1234);
    assert_eq!(paula.adkconr_debug(), 0x1234);
    assert_eq!(paula.pot0dat(), 0x2345);
    assert_eq!(paula.pot0dat_debug(), 0x2345);
    assert_eq!(paula.pot1dat(), 0x3456);
    assert_eq!(paula.pot1dat_debug(), 0x3456);
    assert_eq!(paula.potgor(), 0x4567);
    assert_eq!(paula.potgor_debug(), 0x4567);
    assert_eq!(paula.serdatr(), 0x5678);
    assert_eq!(paula.serdatr_debug(), 0x5678);
    assert_eq!(paula.intenar(), 0x6789);
    assert_eq!(paula.intenar_debug(), 0x6789);
    assert_eq!(paula.intreqr(), 0x789A);
    assert_eq!(paula.intreqr_debug(), 0x789A);
}

#[test]
fn test_dsklen_two_write_arming_sequence() {
    let mut paula = Paula::new();

    // Initial state: not armed, not active
    assert!(!paula.dma_armed);
    assert!(!paula.is_dsk_dma_active());

    // Write 1: length with DMAEN bit 15 = 1 arms the controller
    paula.write_dsklen(0x8100);
    assert!(paula.dma_armed);
    assert!(!paula.is_dsk_dma_active());

    // Write 2: second write starts the transfer
    paula.write_dsklen(0x8100);
    assert!(paula.dma_armed);
    assert!(paula.is_dsk_dma_active());

    // Write 3: clearing bit 15 unarms and stops transfer
    paula.write_dsklen(0x4000);
    assert!(!paula.dma_armed);
    assert!(!paula.is_dsk_dma_active());
}

#[test]
fn test_paula_dskbytr_native_methods() {
    let mut paula = Paula::new();
    paula.dma_master = true;
    paula.dma_enables = paula::DSKBYTR_DSKEN;
    paula.dsklen = paula::DSKLEN_WRITE_FLAG;

    // Latch byte $42 with sync matched true
    paula.set_disk_byte(0x42, true);

    // Expected composite: DSKBYT (0x8000) | DMAON (0x4000) | DISKWRITE (0x2000) | WORDEQUAL (0x1000) | 0x42
    assert_eq!(paula.peek_dskbytr(), 0xF042);
    assert_eq!(paula.read_register(0x01A), 0xF042);
    // Ensure peek did not clear bit 15
    assert_eq!(paula.dskbytr & 0x8000, 0x8000);

    // Read with Clear-on-Read
    assert_eq!(paula.read_dskbytr(), 0xF042);
    // Bit 15 is now cleared in live register
    assert_eq!(paula.dskbytr & 0x8000, 0);
    // Subsequent peek shows 0x7042
    assert_eq!(paula.peek_dskbytr(), 0x7042);
}
