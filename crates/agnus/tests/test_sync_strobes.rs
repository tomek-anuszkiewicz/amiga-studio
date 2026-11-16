mod common;
use agnus::{Agnus, AgnusModel};
use common::TestChipRam;
use config::custom_reg::denise as reg;

#[test]
fn refresh_slots_drive_address_strobes_instead_of_stored_register_values() {
    // HRM Appendix A: a first-slot STREQU/STRVBL/STRHOR code and optional second-slot LONG.
    for model in [AgnusModel::OcsPal8371, AgnusModel::OcsNtsc8370] {
        let mut agnus = Agnus::new(model);
        let mut step_ram_agnus = TestChipRam::default();
        for (line, address) in [
            (0, reg::REGISTER_STREQU),
            (20, reg::REGISTER_STRVBL),
            (100, reg::REGISTER_STRHOR),
        ] {
            agnus.vpos = line;
            agnus.hpos = 0;
            step_ram_agnus.next_cck();
            agnus.step_cck(&mut step_ram_agnus);
            assert_eq!(agnus.pins.rga_strobe, Some(address));
            step_ram_agnus.next_cck();
            agnus.step_cck(&mut step_ram_agnus);
            assert_eq!(agnus.pins.rga_strobe, None, "the address is a timed event");
        }
        agnus.hpos = 2;
        agnus.lol = true;
        step_ram_agnus.next_cck();
        agnus.step_cck(&mut step_ram_agnus);
        assert_eq!(
            agnus.pins.rga_strobe,
            if model == AgnusModel::OcsNtsc8370 {
                Some(reg::REGISTER_STRLONG)
            } else {
                None
            }
        );
        step_ram_agnus.next_cck();
        agnus.step_cck(&mut step_ram_agnus);
        assert_eq!(agnus.pins.rga_strobe, None);
        agnus.reset();
        assert_eq!(agnus.pins.rga_strobe, None);
    }
}
