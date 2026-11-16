mod common;
use agnus::{Agnus, AgnusModel};
use common::TestChipRam;

#[test]
fn queued_audio_locations_discard_unused_address_bits() {
    let mut agnus = Agnus::new(AgnusModel::OcsPal8371);
    let mut step_ram_agnus = TestChipRam::default();
    for channel in 0..4 {
        assert!(agnus.set_audlch(channel, 0x001d));
        assert!(agnus.set_audlcl(channel, 0x2345));
        assert_eq!(agnus.audlc[channel], 0);
    }
    step_ram_agnus.next_cck();
    agnus.step_cck(&mut step_ram_agnus);
    assert_eq!(agnus.audlc, [0x0005_2344; 4]);
    assert_eq!(agnus.audpt, [0x0005_2344; 4]);
}
