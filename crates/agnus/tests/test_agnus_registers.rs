use agnus::{Agnus, AgnusModel, PAL_LINE_CCKS, VHPOSR_PIPELINE_LEAD_CCKS};

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

    // Initial DMACON channel bits are 0
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0);

    // Write at cycle T: SET DMAEN and BLTEN (bit 9 + bit 6 = 0x8240)
    agnus.write_register(0x096, 0x8240);

    // Cycle T: Read is NOW (still 0 because mutation is in flight)
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0);

    // Step 1 CCK (Cycle T+1): still in flight
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0);

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
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0);

    // Write 2 at cycle T+1 before Write 1 commits: SET BPLEN (0x8100) instead
    agnus.write_register(0x096, 0x8100);

    // 1 CCK passes: timer was reset to 2, so at T+2 it has 1 CCK remaining
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0);

    // 2nd CCK after overwrite: now it matures with the latest value (0x8100)
    agnus.step_cck();
    assert_eq!(agnus.read_dmaconr() & 0x07FF, 0x0100);
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

#[test]
fn test_vhposr_beam_lead_and_parity() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    // At hpos = 0, vhposr reflects internal Agnus pipeline lead of 5 CCKs (vAmiga peekVHPOSR)
    let val0 = agnus.vhposr();
    assert_eq!(val0 & 0x00FF, VHPOSR_PIPELINE_LEAD_CCKS);

    // Step 1 CCK (hpos = 1) -> vhposr must reflect 1 + VHPOSR_PIPELINE_LEAD_CCKS
    agnus.step_cck();
    let val1 = agnus.vhposr();
    assert_eq!(val1 & 0x00FF, 1 + VHPOSR_PIPELINE_LEAD_CCKS);
}

#[test]
fn test_vposr_and_vhposr_unified_pipeline_lead() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    // Both VHPOSR and VPOSR must consistently sample the +5 CCK pipelined beam readout
    let vhposr = agnus.vhposr();
    let vposr = agnus.vposr();
    assert_eq!(vhposr & 0x00FF, VHPOSR_PIPELINE_LEAD_CCKS);
    // At vpos = 0, V8 is 0
    assert_eq!(vposr & 0x0007, 0);

    // Advance to scanline 256 where V8 transitions to 1
    for _ in 0..(256 * PAL_LINE_CCKS as usize) {
        agnus.step_cck();
    }
    // Now vpos = 256, so V8 bit 0 in VPOSR must be 1
    let vposr_256 = agnus.vposr();
    assert_eq!(vposr_256 & 0x0001, 1);
}

#[test]
fn test_agnus_canonical_register_constants() {
    use config::custom_reg;
    use config::mask::dmacon;

    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    // Verify write_register works with canonical constants
    agnus.commit_register_write(
        custom_reg::DMACON,
        dmacon::SET_CLR | dmacon::DMAEN | dmacon::COPEN,
    );
    assert!(agnus.is_dma_enabled(dmacon::COPEN));
    assert_eq!(
        agnus.read_register(custom_reg::DMACONR) & dmacon::DMAEN,
        dmacon::DMAEN
    );
}

#[test]
fn test_agnus_ignores_denise_only_registers() {
    let agnus = Agnus::new(AgnusModel::OcsPal8371);
    // DIWSTRT ($08E), DIWSTOP ($090), and BPLCON1 ($102) are Denise-only registers per HRM Appendix A.
    // Reading them on Agnus returns open bus 0xFFFF.
    assert_eq!(agnus.read_register(0x08E), 0xFFFF);
    assert_eq!(agnus.read_register(0x090), 0xFFFF);
    assert_eq!(agnus.read_register(0x102), 0xFFFF);
}

#[test]
fn test_agnus_copper_strobe_copjmp() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    agnus.copper.cop1lc = 0x0004_0000;
    agnus.copper.cop2lc = 0x0006_0000;

    agnus.strobe_copjmp1();
    assert_eq!(agnus.copper.cop_pc, 0x0004_0000);

    agnus.strobe_copjmp2();
    assert_eq!(agnus.copper.cop_pc, 0x0006_0000);

    assert_eq!(agnus.dmaconr(), agnus.dmaconr_debug());
    assert_eq!(agnus.vposr(), agnus.vposr_debug());
    assert_eq!(agnus.vhposr(), agnus.vhposr_debug());
}

#[test]
fn test_is_dma_enabled_requires_master_and_channel() {
    use config::mask::dmacon;

    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    // Channel set, but master DMAEN cleared
    agnus.commit_register_write(0x096, dmacon::SET_CLR | dmacon::COPEN);
    assert!(
        !agnus.is_dma_enabled(dmacon::COPEN),
        "DMA must be false when master DMAEN is off"
    );

    // Enable master DMAEN
    agnus.commit_register_write(0x096, dmacon::SET_CLR | dmacon::DMAEN);
    assert!(
        agnus.is_dma_enabled(dmacon::COPEN),
        "DMA must be true when master DMAEN and channel are on"
    );

    // Disable channel
    agnus.commit_register_write(0x096, dmacon::COPEN);
    assert!(
        !agnus.is_dma_enabled(dmacon::COPEN),
        "DMA must be false when channel is cleared"
    );
}
