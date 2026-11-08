use crate::bessel::{kaiser_window, modified_bessel_i0};

#[test]
fn test_bessel_i0_known_values() {
    // I_0(0) == 1.0
    assert!((modified_bessel_i0(0.0) - 1.0).abs() < 1e-12);
    // I_0(1) ≈ 1.2660658777520084
    assert!((modified_bessel_i0(1.0) - 1.2660658777520084).abs() < 1e-8);
    // Symmetry: I_0(-x) == I_0(x)
    assert_eq!(modified_bessel_i0(2.5), modified_bessel_i0(-2.5));
}

#[test]
fn test_kaiser_window_endpoints() {
    let w = kaiser_window(2048, 8.0);
    assert_eq!(w.len(), 2048);
    // Window should peak at center (close to 1.0) and be symmetric
    let center = w.len() / 2;
    assert!((w[center] - 1.0).abs() < 0.01);
    assert!((w[0] - w[w.len() - 1]).abs() < 1e-12);
}
