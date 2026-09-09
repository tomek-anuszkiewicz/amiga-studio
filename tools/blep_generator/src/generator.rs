//! BLEP (Band-Limited Step) synthesis pipeline for Paula audio DAC anti-aliasing.
//!
//! Generates minimum-phase bandlimited step response tables for Paula's audio channels.

use crate::bessel::kaiser_window;
use crate::biquad::BiquadFilter;
use crate::complex::Complex64;
use crate::config::{AmigaAudioFilterParams, LedFilter, TvMode};
use crate::fft::{fft_in_place, ifft_in_place};
use std::f64::consts::PI;

/// Default BLEP table length in samples.
pub const BLEP_TABLE_LENGTH: usize = 2048;

/// Default anti-aliasing low-pass cutoff frequency in Hz.
pub const BLEP_CUTOFF_FREQUENCY_HZ: f64 = 21000.0;

/// Normalized sinc function: $\text{sinc}(x) = \frac{\sin(\pi x)}{\pi x}$, $\text{sinc}(0) = 1$.
#[inline]
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-15 {
        1.0
    } else {
        let px = PI * x;
        px.sin() / px
    }
}

/// Generates a bandlimited impulse response sampled at `sampling_rate`.
pub fn create_bandlimited_impulse(sampling_rate: f64, length: usize, cutoff_freq: f64) -> Vec<f64> {
    let t_period = 1.0 / sampling_rate;
    let mut result = Vec::with_capacity(length);
    let center_offset = t_period * ((length + 1) as f64) / 2.0;

    for i in 0..length {
        let t = t_period * ((i + 1) as f64) - center_offset;
        let b = 2.0 * cutoff_freq;
        result.push(sinc(b * t));
    }

    let sum: f64 = result.iter().sum();
    for sample in result.iter_mut() {
        *sample /= sum;
    }

    result
}

/// Safe logarithm clamped at -60 dB to prevent $\ln(0) = -\infty$.
#[inline]
fn log_safe(x: f64) -> f64 {
    if x <= 0.0 {
        -60.0
    } else {
        x.ln()
    }
}

/// Transforms a symmetric linear-phase FIR impulse response into a causal, minimum-phase filter
/// via the real cepstrum algorithm.
///
/// ### Why Minimum Phase (Physical Causality):
/// Symmetric linear-phase filters exhibit "pre-ringing" — oscillation begins *before* the impulse/step event.
/// In a real physical audio circuit, causality requires that the response can only happen *after* (or at)
/// the DAC step transition ($t \ge 0$), never in the past ($t < 0$). Converting to minimum phase packs
/// all energy to the start of the impulse response, strictly eliminating anticausal pre-ringing.
///
/// ### Steps:
/// 1. Zero-pad table to $8 \times N$ ($16384$ samples) for smooth frequency resolution.
/// 2. Compute the real cepstrum: $\text{cepstrum} = \text{IFFT}(\ln |\text{FFT}(h)|)$.
/// 3. Window the cepstrum to eliminate anticausal components (double causal, zero anticausal).
/// 4. Invert the cepstrum: $h_{\text{min}} = \text{IFFT}(\exp(\text{FFT}(\text{cepstrum})))$.
pub fn transform_into_minimum_phase(signal: &[f64], length: usize) -> Vec<f64> {
    let pad_length = length * 8;
    assert!(pad_length.is_power_of_two());

    let mut padded = vec![Complex64::ZERO; pad_length];
    for (i, &s) in signal.iter().enumerate().take(length) {
        padded[i] = Complex64::from_real(s);
    }

    // 1. FFT
    fft_in_place(&mut padded);

    // 2. Real cepstrum: log-magnitude followed by IFFT
    let mut cepstrum: Vec<Complex64> = padded
        .iter()
        .map(|z| Complex64::from_real(log_safe(z.magnitude())))
        .collect();
    ifft_in_place(&mut cepstrum);

    // 3. Window cepstrum: reject anticausal components
    let half = cepstrum.len() / 2;
    for i in 1..half {
        cepstrum[i] *= 2.0;
    }
    for i in (half + 1)..cepstrum.len() {
        cepstrum[i] = Complex64::ZERO;
    }

    // 4. Exponentiate spectrum and return to time domain
    fft_in_place(&mut cepstrum);
    for z in cepstrum.iter_mut() {
        *z = z.exp();
    }
    ifft_in_place(&mut cepstrum);

    cepstrum[..length].iter().map(|z| z.re).collect()
}

/// Integrates a bandlimited impulse response to obtain a bandlimited step (BLEP).
///
/// Running sum starts at $-\sum h[n]$ and accumulates forward to 0.
pub fn integrate_step(signal: &mut [f64]) {
    let total: f64 = signal.iter().sum();
    let mut start_val = -total;
    for sample in signal.iter_mut() {
        start_val += *sample;
        *sample = start_val;
    }
}

/// Quantizes and normalizes the continuous step curve to full scale $2^{17} = 131072$.
pub fn quantize_and_scale(signal: &[f64]) -> Vec<i32> {
    let fact = 131072.0; // 2^17
    let first = signal.first().copied().unwrap_or(0.0);
    let last = signal.last().copied().unwrap_or(1.0);
    let correction_factor = last - first;

    let mut result = Vec::with_capacity(signal.len());
    for &x in signal {
        let val = x * fact / correction_factor;
        let rounded = if val < 0.0 {
            (val - 0.5) as i64
        } else {
            (val + 0.5) as i64
        };
        result.push((-rounded) as i32);
    }

    result
}

/// Synthesizes a complete BLEP table for the specified Amiga hardware filter configuration.
pub fn generate_blep(
    params: &AmigaAudioFilterParams,
    tv_mode: TvMode,
    led_filter: LedFilter,
) -> Vec<i32> {
    let sampling_rate = tv_mode.sampling_rate();

    // 1. Bandlimited sinc impulse
    let mut signal =
        create_bandlimited_impulse(sampling_rate, BLEP_TABLE_LENGTH, BLEP_CUTOFF_FREQUENCY_HZ);

    // 2. Kaiser windowing
    let kaiser = kaiser_window(BLEP_TABLE_LENGTH, params.kaiser_window_beta);
    for (s, &w) in signal.iter_mut().zip(kaiser.iter()) {
        *s *= w;
    }

    // 3. Minimum phase reconstruction
    let mut signal = transform_into_minimum_phase(&signal, BLEP_TABLE_LENGTH);

    // 4. Analog circuit filtering
    // 4a. 2nd-order Sallen-Key LED filter (if active)
    if led_filter == LedFilter::On {
        let den_s2 = params.r332 * params.r333 * params.c333 * params.c332;
        let den_s1 = params.c333 * (params.r332 + params.r333);
        let mut biquad =
            BiquadFilter::from_continuous_transfer_function(den_s2, den_s1, sampling_rate);
        biquad.filter_signal(&mut signal);
    }

    // 4b. 1st-order RC low-pass filter (fixed stage)
    let den_s2 = 0.0;
    let den_s1 = params.r331 * params.c331;
    let mut lowpass_biquad =
        BiquadFilter::from_continuous_transfer_function(den_s2, den_s1, sampling_rate);
    lowpass_biquad.filter_signal(&mut signal);

    // 5. Numerical step integration
    integrate_step(&mut signal);

    // 6. Scale and quantize to 18-bit fixed-point format (range ~ [0, 131072])
    quantize_and_scale(&signal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinc_zero() {
        assert_eq!(sinc(0.0), 1.0);
    }

    #[test]
    fn test_blep_generation_bounds() {
        let a500_params = AmigaAudioFilterParams::a500();
        let blep = generate_blep(&a500_params, TvMode::Pal, LedFilter::Off);
        assert_eq!(blep.len(), BLEP_TABLE_LENGTH);
        // First elements should be around 131072 (full scale)
        assert!((blep[0] - 131072).abs() < 10);
        // Last elements should settle near 0
        assert!(blep[blep.len() - 1].abs() < 10);
    }
}
