use denise::{Denise, DeniseModel};

#[test]
fn test_clxdat_clear_on_read_vs_peek() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.clxdat = 0x5A5A;

    // peek_register should NOT clear CLXDAT
    assert_eq!(denise.peek_register(0x00E), 0x5A5A);
    assert_eq!(denise.peek_register(0x00E), 0x5A5A);

    // read_register MUST clear CLXDAT immediately on read
    assert_eq!(denise.read_register(0x00E), 0x5A5A);
    assert_eq!(denise.read_register(0x00E), 0x0000);
}

#[test]
fn test_color_write_immediate_commit_active_cycle() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);

    // Write COLOR00 ($180) = 0x0F00 (pure red)
    let res = denise.write_register(0x180, 0x0F00);

    // Cycle T: Color DAC palette updates immediately on the active cycle
    assert_eq!(res, Some((0x180, 0x0F00)));
    assert_eq!(denise.read_color(0), 0x0F00);

    // Step 1 CCK (Cycle T+1): color remains active
    denise.step_cck(config::BeamPosition::default());
    assert_eq!(denise.read_color(0), 0x0F00);
}

#[test]
fn test_bplcon0_denise_propagation_delay_1_cck() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);

    // Write BPLCON0 ($100) = 0x8200 (HIRES mode)
    denise.write_register(0x100, 0x8200);

    // Before step: still 0
    assert_eq!(denise.bplcon0, 0);

    // After 1 CCK: commits
    denise.step_cck(config::BeamPosition::default());
    assert_eq!(denise.bplcon0, 0x8200);
}

#[test]
fn test_write_only_registers_read_open_bus() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    assert_eq!(denise.read_register(0x100), 0xFFFF);
    assert_eq!(denise.read_register(0x180), 0xFFFF);
}
