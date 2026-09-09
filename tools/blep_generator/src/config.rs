//! Hardware configuration and circuit presets for Amiga audio filtering.
//!
//! Defines physical resistor/capacitor component parameters for the Paula DAC output stage:
//! - Fixed low-pass RC filter (R331, C331).
//! - Switchable Sallen-Key 2nd-order filter (R332, R333, C332, C333, controlled by CIA-A LED bit).

/// Hardware circuit presets for Amiga audio low-pass filtering.
#[derive(Debug, Clone)]
pub struct AmigaAudioFilterParams {
    pub kaiser_window_beta: f64,
    /// Capacitance C331 (in Farads) for fixed low-pass filter.
    pub c331: f64,
    /// Resistance R331 (in Ohms) for fixed low-pass filter.
    pub r331: f64,
    /// Resistance R332 (in Ohms) for Sallen-Key LED filter.
    pub r332: f64,
    /// Resistance R333 (in Ohms) for Sallen-Key LED filter.
    pub r333: f64,
    /// Capacitance C332 (in Farads) for Sallen-Key LED filter.
    pub c332: f64,
    /// Capacitance C333 (in Farads) for Sallen-Key LED filter.
    pub c333: f64,
}

impl AmigaAudioFilterParams {
    /// Reference parameters for Amiga 500 (OCS/ECS) matching WinUAE.
    ///
    /// Fixed filter cutoff: $f_c \approx \frac{1}{2\pi R_{331} C_{331}} \approx 4900\,\text{Hz}$.
    /// Sallen-Key filter cutoff: $f_0 \approx 3275\,\text{Hz}$, $Q \approx 0.65$.
    pub fn a500() -> Self {
        Self {
            kaiser_window_beta: 8.0,
            c331: 9.0223890641664011e-8,
            r331: 360.0,
            r332: 10000.0,
            r333: 10000.0,
            c332: 6.3404864595739559e-9,
            c333: 3.724773897250293e-9,
        }
    }

    /// Reference parameters for Amiga 1200 (AGA) matching WinUAE.
    ///
    /// Fixed filter cutoff: $f_c \approx 32000\,\text{Hz}$ (leakage / higher cutoff).
    /// Sallen-Key filter cutoff: $f_0 \approx 3275\,\text{Hz}$.
    pub fn a1200() -> Self {
        Self {
            kaiser_window_beta: 9.0,
            c331: 7.31410584062019e-9,
            r331: 680.0,
            r332: 10000.0,
            r333: 10000.0,
            c332: 6.3404864595739559e-9,
            c333: 3.724773897250293e-9,
        }
    }
}

/// State of the audio LED filter switch (controlled by CIA-A bit 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedFilter {
    Off,
    On,
}

/// Television video / master clock mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvMode {
    Pal,
    #[allow(dead_code)]
    Ntsc,
}

impl TvMode {
    /// Audio DAC sampling rate in Hz (half master clock).
    #[inline]
    pub fn sampling_rate(self) -> f64 {
        match self {
            TvMode::Pal => 7093790.0 / 2.0,  // ~3.546895 MHz
            TvMode::Ntsc => 7159090.0 / 2.0, // ~3.579545 MHz
        }
    }
}
