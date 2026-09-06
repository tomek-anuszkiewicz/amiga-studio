//! Amiga 500 Configuration Model (`A500Config`)
//!
//! Encapsulated, read-only configuration model for the Amiga 500 emulator.
//! State mutation is restricted exclusively to applying canonical hardware presets.

use serde::{Deserialize, Serialize};

/// Canonical hardware configuration presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum A500Preset {
    /// Preset 1: Bare Stock A500 (512 KB Chip RAM, no expansions, no RTC)
    Bare512k,
    /// Preset 2: Standard A500 + A501 (512 KB Chip + 512 KB Slow RAM + MSM6242B RTC)
    Standard1Mb,
    /// Preset 3: Power User A500 (512 KB Chip + 512 KB Slow + 4 MB Fast RAM + MSM6242B RTC)
    ExpandedPowerUser,
}

/// Video standard and master clock timing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoStandard {
    /// PAL (50 Hz refresh, ~3.546895 MHz Color Clock)
    Pal,
    /// NTSC (60 Hz refresh, ~3.579545 MHz Color Clock)
    Ntsc,
}

/// Real-Time Clock hardware model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RtcModel {
    /// No RTC installed (returns floating bus $FF at $DC0000..$DC003F)
    None,
    /// OKI MSM6242B (standard on A501 expansion, A500+, and A2000)
    Msm6242b,
}

/// Chip RAM size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipRamSize {
    /// Standard baseline 512 KB Chip RAM ($000000 - $07FFFF)
    Kb512,
}

/// Slow / Trapdoor RAM size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlowRamSize {
    /// No Slow RAM installed
    None,
    /// 512 KB Trapdoor Slow RAM at $C00000 ($C00000 - $C7FFFF)
    Kb512,
}

/// Fast RAM expansion size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FastRamSize {
    /// No Fast RAM installed
    None,
    /// 4 MB Auto-Config Fast RAM at $200000 ($200000 - $5FFFFF)
    Mb4,
}

/// Master configuration struct for the Amiga 500 emulator.
///
/// Internal fields are private and immutable from outside. Mutation is strictly
/// constrained to selecting canonical hardware presets via [`A500Config::apply_preset`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A500Config {
    preset: A500Preset,
    video_standard: VideoStandard,
    chip_ram: ChipRamSize,
    slow_ram: SlowRamSize,
    fast_ram: FastRamSize,
    rtc: RtcModel,
}

impl Default for A500Config {
    /// Default configuration is the Golden Standard A500 + A501 (1 MB + RTC, PAL)
    fn default() -> Self {
        Self::standard_1mb(VideoStandard::Pal)
    }
}

impl A500Config {
    /// Creates a configuration from a preset and video standard
    pub fn from_preset(preset: A500Preset, video: VideoStandard) -> Self {
        let mut config = Self {
            preset,
            video_standard: video,
            chip_ram: ChipRamSize::Kb512,
            slow_ram: SlowRamSize::None,
            fast_ram: FastRamSize::None,
            rtc: RtcModel::None,
        };
        config.apply_preset(preset);
        config
    }

    /// Preset 1: Bare Stock A500 (512 KB Chip RAM, no expansions, no RTC)
    pub fn bare_512k(video: VideoStandard) -> Self {
        Self::from_preset(A500Preset::Bare512k, video)
    }

    /// Preset 2: Standard A500 + A501 (512 KB Chip + 512 KB Slow RAM + MSM6242B RTC)
    pub fn standard_1mb(video: VideoStandard) -> Self {
        Self::from_preset(A500Preset::Standard1Mb, video)
    }

    /// Preset 3: Power User A500 (512 KB Chip + 512 KB Slow + 4 MB Fast RAM + MSM6242B RTC)
    pub fn expanded_power_user(video: VideoStandard) -> Self {
        Self::from_preset(A500Preset::ExpandedPowerUser, video)
    }

    /// The only mutation method: applies one of the predefined hardware presets.
    pub fn apply_preset(&mut self, preset: A500Preset) {
        self.preset = preset;
        match preset {
            A500Preset::Bare512k => {
                self.chip_ram = ChipRamSize::Kb512;
                self.slow_ram = SlowRamSize::None;
                self.fast_ram = FastRamSize::None;
                self.rtc = RtcModel::None;
            }
            A500Preset::Standard1Mb => {
                self.chip_ram = ChipRamSize::Kb512;
                self.slow_ram = SlowRamSize::Kb512;
                self.fast_ram = FastRamSize::None;
                self.rtc = RtcModel::Msm6242b;
            }
            A500Preset::ExpandedPowerUser => {
                self.chip_ram = ChipRamSize::Kb512;
                self.slow_ram = SlowRamSize::Kb512;
                self.fast_ram = FastRamSize::Mb4;
                self.rtc = RtcModel::Msm6242b;
            }
        }
    }

    // --- Read-Only Getters ---

    /// Returns the currently active hardware preset
    #[inline]
    pub fn active_preset(&self) -> A500Preset {
        self.preset
    }

    /// Returns the configured video standard (PAL or NTSC)
    #[inline]
    pub fn video_standard(&self) -> VideoStandard {
        self.video_standard
    }

    /// Returns the configured Chip RAM size
    #[inline]
    pub fn chip_ram(&self) -> ChipRamSize {
        self.chip_ram
    }

    /// Returns the configured Slow RAM size
    #[inline]
    pub fn slow_ram(&self) -> SlowRamSize {
        self.slow_ram
    }

    /// Returns the configured Fast RAM size
    #[inline]
    pub fn fast_ram(&self) -> FastRamSize {
        self.fast_ram
    }

    /// Returns the Real-Time Clock model
    #[inline]
    pub fn rtc(&self) -> RtcModel {
        self.rtc
    }
}
