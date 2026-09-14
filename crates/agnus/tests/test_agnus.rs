use agnus::{Agnus, AgnusModel, PAL_LINE_CCKS};

#[test]
fn test_agnus_beam_progression() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    assert_eq!(agnus.hpos, 0);
    assert_eq!(agnus.vpos, 0);

    // Step through one full horizontal scanline
    for _ in 0..=PAL_LINE_CCKS {
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
