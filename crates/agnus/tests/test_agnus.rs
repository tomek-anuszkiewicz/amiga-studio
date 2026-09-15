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

#[test]
fn test_agnus_pal_scanline_length_and_frame_total() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    assert_eq!(agnus.hpos, 0);
    assert_eq!(agnus.vpos, 0);
    assert!(!agnus.lol);

    // In PAL, all lines have strictly 227 CCKs (PAL_LINE_CCKS)
    for _ in 0..227 {
        agnus.step_cck();
    }
    assert_eq!(agnus.vpos, 1);
    assert_eq!(agnus.hpos, 0);
    assert!(!agnus.lol);

    // Line 1 also has 227 CCKs
    for _ in 0..227 {
        agnus.step_cck();
    }
    assert_eq!(agnus.vpos, 2);
    assert_eq!(agnus.hpos, 0);
    assert!(!agnus.lol);

    // Total CCKs for the 312 lines in the frame: 312 * 227 = 70,824
    let mut total_ccks = 227 + 227;
    while agnus.vpos != 0 {
        agnus.step_cck();
        total_ccks += 1;
    }
    assert_eq!(total_ccks, 70_824);
}

#[test]
fn test_agnus_ntsc_lol_alternation() {
    let mut agnus = Agnus::new(AgnusModel::OcsNtsc8370);
    assert_eq!(agnus.hpos, 0);
    assert_eq!(agnus.vpos, 0);
    assert!(!agnus.lol);

    // Line 0 (even) has 227 CCKs
    for _ in 0..227 {
        agnus.step_cck();
    }
    assert_eq!(agnus.vpos, 1);
    assert_eq!(agnus.hpos, 0);
    assert!(agnus.lol);

    // Line 1 (odd) has 228 CCKs
    for _ in 0..228 {
        agnus.step_cck();
    }
    assert_eq!(agnus.vpos, 2);
    assert_eq!(agnus.hpos, 0);
    assert!(!agnus.lol);
}

#[test]
fn test_agnus_bpl_dma_polling() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    let mut chip_ram = vec![0u8; 0x40000];
    chip_ram[0x1000] = 0xAA;
    chip_ram[0x1001] = 0x55;

    agnus.bplpt[0] = 0x1000;
    // Enable Master DMA + Bitplane DMA ($8300)
    agnus.commit_register_write(0x096, 0x8300);
    // Configure 1 bitplane in BPLCON0 ($1200)
    agnus.set_bplcon0(0x1200);
    // DDF window at slot 0x38
    agnus.ddfstrt = 0x38;
    agnus.ddfstop = 0xD0;

    agnus.hpos = 0x37;
    agnus.vpos = 50;
    agnus.step_cck_ram(&mut chip_ram);

    // Slot 0x38 should have fetched plane 0 from Chip RAM
    assert_eq!(agnus.poll_bpl_dma(), Some((0, 0xAA55)));
    assert_eq!(agnus.bplpt[0], 0x1002);
}
