//! In-place Radix-2 Cooley-Tukey Fast Fourier Transform (FFT).

use crate::complex::Complex64;
use std::f64::consts::PI;

/// Computes the in-place forward Radix-2 Fast Fourier Transform (FFT).
///
/// The length of `buffer` must be a power of two.
pub fn fft_in_place(buffer: &mut [Complex64]) {
    let n = buffer.len();
    assert!(n > 0 && n.is_power_of_two(), "FFT length must be a power of 2");

    // 1. Bit-reversal permutation
    let mut j = 0;
    for i in 0..n {
        if i < j {
            buffer.swap(i, j);
        }
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
    }

    // 2. Cooley-Tukey butterflies
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = -2.0 * PI / (len as f64);
        let w_step = Complex64::new(angle.cos(), angle.sin());

        let mut i = 0;
        while i < n {
            let mut w = Complex64::ONE;
            for k in 0..half {
                let u = buffer[i + k];
                let v = buffer[i + k + half] * w;
                buffer[i + k] = u + v;
                buffer[i + k + half] = u - v;
                w = w * w_step;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// Computes the in-place inverse Fast Fourier Transform (IFFT).
///
/// Uses the property: $\text{IFFT}(x) = \frac{1}{N} (\text{FFT}(x^*))^*$.
pub fn ifft_in_place(buffer: &mut [Complex64]) {
    let n = buffer.len();
    assert!(n > 0 && n.is_power_of_two(), "IFFT length must be a power of 2");

    for sample in buffer.iter_mut() {
        *sample = sample.conj();
    }

    fft_in_place(buffer);

    let n_f64 = n as f64;
    for sample in buffer.iter_mut() {
        *sample = sample.conj() / n_f64;
    }
}

/// Convenience forward FFT returning a new vector.
#[allow(dead_code)]
pub fn fft(input: &[Complex64]) -> Vec<Complex64> {
    let mut buf = input.to_vec();
    fft_in_place(&mut buf);
    buf
}

/// Convenience inverse FFT returning a new vector.
#[allow(dead_code)]
pub fn ifft(input: &[Complex64]) -> Vec<Complex64> {
    let mut buf = input.to_vec();
    ifft_in_place(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_ifft_roundtrip() {
        let original: Vec<Complex64> = (0..16)
            .map(|i| Complex64::new((i as f64).sin(), (i as f64 * 0.5).cos()))
            .collect();

        let transformed = fft(&original);
        let restored = ifft(&transformed);

        for (a, b) in original.iter().zip(restored.iter()) {
            assert!((a.re - b.re).abs() < 1e-12, "Re mismatch: {} vs {}", a.re, b.re);
            assert!((a.im - b.im).abs() < 1e-12, "Im mismatch: {} vs {}", a.im, b.im);
        }
    }

    #[test]
    fn test_delta_impulse_spectrum() {
        // Delta impulse at 0 should have flat frequency response of magnitude 1
        let mut delta = vec![Complex64::ZERO; 8];
        delta[0] = Complex64::ONE;
        let spectrum = fft(&delta);
        for s in spectrum {
            assert!((s.magnitude() - 1.0).abs() < 1e-12);
        }
    }
}
