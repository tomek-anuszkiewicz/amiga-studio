use agnus::{Agnus, AgnusModel};

#[test]
fn test_dmacon_set_clr_logic() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // Immediate commit helper for testing logic
    agnus.commit_register_write(0x096, 0x8200); // SET DMAEN (bit 9)
    assert_eq!(agnus.dmacon & 0x0200, 0x0200);

    agnus.commit_register_write(0x096, 0x8020); // SET SPREN (bit 5)
    assert_eq!(agnus.dmacon & 0x0220, 0x0220);

    agnus.commit_register_write(0x096, 0x0020); // CLR SPREN (bit 5)
    assert_eq!(agnus.dmacon & 0x0020, 0);
    assert_eq!(agnus.dmacon & 0x0200, 0x0200); // DMAEN still set
}

#[test]
fn test_dmacon_propagation_delay_2_cck() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // Initial DMACON is 0
    assert_eq!(agnus.read_dmaconr(), 0);

    // Write at cycle T: SET DMAEN and BLTEN (bit 9 + bit 6 = 0x8240)
    agnus.write_register(0x096, 0x8240);

    // Cycle T: Read is NOW (still 0 because mutation is in flight)
    assert_eq!(agnus.read_dmaconr(), 0);

    // Step 1 CCK (Cycle T+1): still in flight
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr(), 0);

    // Step 2 CCK (Cycle T+2): mutation matures and commits!
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr() & 0x0240, 0x0240);
}

#[test]
fn test_bplcon0_agnus_propagation_delay_4_cck() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // Write BPLCON0 = 0x5200 (5 bitplanes)
    agnus.write_register(0x100, 0x5200);

    // Should take 4 CCKs to commit to Agnus DMA sequencer
    for _ in 0..3 {
        assert_eq!(agnus.bplcon0, 0);
        agnus.step_cck();
    }
    assert_eq!(agnus.bplcon0, 0);

    // 4th CCK step commits
    agnus.step_cck();
    assert_eq!(agnus.bplcon0, 0x5200);
}

#[test]
fn test_dmacon_overwrite_pending() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // Write 1 at cycle T: SET DMAEN (0x8200)
    agnus.write_register(0x096, 0x8200);

    // 1 CCK passes
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr(), 0);

    // Write 2 at cycle T+1 before Write 1 commits: SET BPLEN (0x8100) instead
    agnus.write_register(0x096, 0x8100);

    // 1 CCK passes: timer was reset to 2, so at T+2 it has 1 CCK remaining
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr(), 0);

    // 2nd CCK after overwrite: now it matures with the latest value (0x8100)
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr(), 0x0100);
}

#[test]
fn test_blitter_and_copper_pointers() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);

    // COP1LCH ($080) = $0007, COP1LCL ($082) = $2000
    agnus.commit_register_write(0x080, 0x0007);
    agnus.commit_register_write(0x082, 0x2000);
    assert_eq!(agnus.copper.cop1lc, 0x0007_2000);

    // BLTAPTH ($050) = $0003, BLTAPTL ($052) = $4566
    agnus.commit_register_write(0x050, 0x0003);
    agnus.commit_register_write(0x052, 0x4566);
    assert_eq!(agnus.blitter.bltapt, 0x0003_4566);
}

#[test]
fn test_write_only_registers_read_open_bus() {
    let agnus = Agnus::new(AgnusModel::OcsPal8371);
    // COP1LCH, BLTCON0, etc. are write-only
    assert_eq!(agnus.read_register(0x080), 0xFFFF);
    assert_eq!(agnus.read_register(0x040), 0xFFFF);
}
