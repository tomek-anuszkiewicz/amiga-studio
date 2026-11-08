use crate::complex::Complex64;
use crate::fft::{fft, ifft};

#[test]
fn test_fft_ifft_roundtrip() {
    let original: Vec<Complex64> = (0..16)
        .map(|i| Complex64::new((i as f64).sin(), (i as f64 * 0.5).cos()))
        .collect();

    let transformed = fft(&original);
    let restored = ifft(&transformed);

    for (a, b) in original.iter().zip(restored.iter()) {
        assert!(
            (a.re - b.re).abs() < 1e-12,
            "Re mismatch: {} vs {}",
            a.re,
            b.re
        );
        assert!(
            (a.im - b.im).abs() < 1e-12,
            "Im mismatch: {} vs {}",
            a.im,
            b.im
        );
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
