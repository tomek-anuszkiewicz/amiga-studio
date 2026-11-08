use crate::complex::Complex64;

#[test]
fn test_complex_arithmetic() {
    let a = Complex64::new(1.0, 2.0);
    let b = Complex64::new(3.0, 4.0);
    assert_eq!(a + b, Complex64::new(4.0, 6.0));
    assert_eq!(a - b, Complex64::new(-2.0, -2.0));
    assert_eq!(a * b, Complex64::new(-5.0, 10.0));
    assert_eq!(a.conj(), Complex64::new(1.0, -2.0));
}

#[test]
fn test_euler_identity() {
    let pi_i = Complex64::new(0.0, std::f64::consts::PI);
    let exp_pi_i = pi_i.exp();
    assert!((exp_pi_i.re + 1.0).abs() < 1e-12);
    assert!(exp_pi_i.im.abs() < 1e-12);
}
