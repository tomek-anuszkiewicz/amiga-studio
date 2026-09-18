use paula::Paula;

#[test]
fn test_intena_intreq_propagation_delay_1_cck() {
    let mut paula = Paula::new();

    // Write INTENA at cycle T (SET INTEN master + Level 3 VBlank)
    paula.write_register(0x09A, 0xC020);

    // At cycle T: not yet committed
    assert_eq!(paula.intena, 0);
    assert_eq!(paula.read_register(0x01C), 0);

    // Step 1 CCK (Cycle T+1): matures and commits!
    paula.step_cck();
    assert_eq!(paula.intena, 0x4020);
    assert_eq!(paula.read_register(0x01C), 0x4020);

    // Write INTREQ at cycle T+1 (SET Level 3 VBlank request)
    paula.write_register(0x09C, 0x8020);
    assert_eq!(paula.intreq, 0);
    assert_eq!(paula.pending_interrupt_level(), 0);

    // Step 1 CCK (Cycle T+2): matures and commits, triggering IPL 3!
    paula.step_cck();
    assert_eq!(paula.intreq, 0x0020);
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
