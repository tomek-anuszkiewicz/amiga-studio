use config::{
    A500Config, A500Preset, AgnusModel, ChipRamSize, DeniseModel, FastRamSize, RtcModel,
    SlowRamSize, VideoStandard,
};

#[test]
fn test_default_config() {
    let config = A500Config::default();
    assert_eq!(config.active_preset(), A500Preset::Standard1Mb);
    assert_eq!(config.video_standard(), VideoStandard::Pal);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::Kb512);
    assert_eq!(config.fast_ram(), FastRamSize::None);
    assert_eq!(config.rtc(), RtcModel::Msm6242b);
    assert_eq!(config.agnus_model(), AgnusModel::OcsPal8371);
    assert_eq!(config.denise_model(), DeniseModel::Ocs8362);
}

#[test]
fn test_bare_512k_preset() {
    let config = A500Config::bare_512k(VideoStandard::Ntsc);
    assert_eq!(config.active_preset(), A500Preset::Bare512k);
    assert_eq!(config.video_standard(), VideoStandard::Ntsc);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::None);
    assert_eq!(config.fast_ram(), FastRamSize::None);
    assert_eq!(config.rtc(), RtcModel::None);
    assert_eq!(config.agnus_model(), AgnusModel::OcsNtsc8370);
    assert_eq!(config.denise_model(), DeniseModel::Ocs8362);
}

#[test]
fn test_expanded_power_user_preset() {
    let config = A500Config::expanded_power_user(VideoStandard::Pal);
    assert_eq!(config.active_preset(), A500Preset::ExpandedPowerUser);
    assert_eq!(config.video_standard(), VideoStandard::Pal);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::Kb512);
    assert_eq!(config.fast_ram(), FastRamSize::Mb4);
    assert_eq!(config.rtc(), RtcModel::Msm6242b);
}
