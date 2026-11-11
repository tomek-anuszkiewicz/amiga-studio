use agnus::{Agnus, AgnusModel};
use config::custom_reg::denise as reg;

#[test]
fn refresh_slots_drive_address_strobes_instead_of_stored_register_values() {
    // HRM Appendix A: a first-slot STREQU/STRVBL/STRHOR code and optional second-slot LONG.
    for model in [AgnusModel::OcsPal8371, AgnusModel::OcsNtsc8370] {
        let mut agnus = Agnus::new(model);
        for (line, address) in [(0, reg::STREQU), (20, reg::STRVBL), (100, reg::STRHOR)] {
            agnus.vpos = line;
            agnus.hpos = 0;
            agnus.step_cck();
            assert_eq!(agnus.pins.rga_strobe, Some(address));
            agnus.step_cck();
            assert_eq!(agnus.pins.rga_strobe, None, "the address is a timed event");
        }
        agnus.hpos = 2;
        agnus.lol = true;
        agnus.step_cck();
        assert_eq!(
            agnus.pins.rga_strobe,
            if model == AgnusModel::OcsNtsc8370 {
                Some(reg::STRLONG)
            } else {
                None
            }
        );
        agnus.step_cck();
        assert_eq!(agnus.pins.rga_strobe, None);
        agnus.reset();
        assert_eq!(agnus.pins.rga_strobe, None);
    }
}
