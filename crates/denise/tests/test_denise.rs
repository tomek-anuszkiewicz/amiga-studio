use denise::{Denise, DeniseModel};

#[test]
fn test_denise_reset_and_palette() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    assert_eq!(denise.read_color(0), 0);

    // Write white: 0x0FFF
    denise.write_color(0, 0x0FFF);
    assert_eq!(denise.read_color(0), 0x0FFF);

    // CLXDAT clear-on-read
    denise.clxdat = 0x1234;
    assert_eq!(denise.read_clxdat(), 0x1234);
    assert_eq!(denise.read_clxdat(), 0);

    denise.reset();
    assert_eq!(denise.read_color(0), 0);
}
