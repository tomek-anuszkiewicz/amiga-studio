use config::DeniseModel;
use denise::Denise;

#[test]
fn repeated_address_strobes_update_receiver_state_immediately() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    denise.sync.hpos = 90;
    denise.sync.vpos = 77;
    denise.strobe_strequ();
    assert_eq!(denise.sync.hpos, 0);
    assert_eq!(denise.sync.vpos, 0);
    assert!(denise.sync.vertical_blank && denise.sync.equalization);
    denise.strobe_strequ();
    assert_eq!(denise.sync.vpos, 1, "the same address can be another event");
    denise.strobe_strvbl();
    assert!(denise.sync.vertical_blank && !denise.sync.equalization);
    denise.strobe_strlong();
    assert!(denise.sync.long_line);
    denise.strobe_strhor();
    assert!(!denise.sync.vertical_blank && !denise.sync.long_line);
    assert!(denise.mutations.iter().all(Option::is_none));
    denise.reset();
    assert!(!denise.sync.synchronized);
}
