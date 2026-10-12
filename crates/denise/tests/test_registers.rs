use denise::{Denise, DeniseModel};

#[test]
fn ocs_reserved_diwhigh_offset_does_not_change_denise_state() {
    let mut denise = Denise::new(DeniseModel::Ocs8362);
    let untouched = denise.clone();

    denise.commit_register_write(0x1E4, 0x2020);

    assert!(denise == untouched, "OCS Denise must ignore reserved $1E4");
}
