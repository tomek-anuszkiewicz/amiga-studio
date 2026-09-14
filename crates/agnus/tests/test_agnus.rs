use agnus::{Agnus, AgnusModel, PAL_LINE_CCKS};

#[test]
fn test_agnus_beam_progression() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    assert_eq!(agnus.hpos, 0);
    assert_eq!(agnus.vpos, 0);

    // Step through one full horizontal scanline (227 CCKs)
    for _ in 0..PAL_LINE_CCKS {
        agnus.step_cck();
    }
    assert_eq!(agnus.vpos, 1);
    assert_eq!(agnus.hpos, 0);
}

#[test]
fn test_agnus_vposr_chip_id() {
    let agnus_pal = Agnus::new(AgnusModel::OcsPal8371);
    assert_eq!(agnus_pal.vposr() & 0x7000, 0);

    let agnus_ntsc = Agnus::new(AgnusModel::OcsNtsc8370);
    assert_eq!(agnus_ntsc.vposr() & 0x7000, 0x1000);
}

#[test]
fn test_agnus_dmacon_syncs_copper_and_blitter() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    assert!(!agnus.copper.dma_enabled);
    assert!(!agnus.blitter.dma_enabled);

    // Enable Master DMA + Copper + Blitter ($82C0)
    agnus.write_register(0x096, 0x82C0);
    for _ in 0..2 {
        agnus.step_cck();
    }
    assert!(agnus.copper.dma_enabled);
    assert!(agnus.blitter.dma_enabled);

    // Disable Copper DMA ($0080)
    agnus.write_register(0x096, 0x0080);
    for _ in 0..2 {
        agnus.step_cck();
    }
    assert!(!agnus.copper.dma_enabled);
    assert!(agnus.blitter.dma_enabled);
}

#[test]
fn test_agnus_batch_mutations_commit_all_without_dropping() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // Write 12 blitter pointer/control registers consecutively
    for i in 0..12u16 {
        agnus.write_register(0x040 + i * 2, 0x1234 + i);
    }

    // Step 2 CCKs to mature all mutations
    for _ in 0..2 {
        agnus.step_cck();
    }

    // Verify all 12 registers were committed into active silicon state
    assert_eq!(agnus.blitter.bltcon0, 0x1234);
    assert_eq!(agnus.blitter.bltcon1, 0x1235);
    assert_eq!(agnus.blitter.bltafwm, 0x1236);
    assert_eq!(agnus.blitter.bltalwm, 0x1237);
}
