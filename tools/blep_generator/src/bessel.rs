//! Modified Bessel function of order 0 ($I_0$) and Kaiser window generation.
//!
//! Based on Chebyshev polynomial approximations from the CERN Cephes mathematical library.

/// Chebyshev coefficients for $\exp(-x) I_0(x)$ in the interval $[0, 8]$.
const I0_COEFFICIENTS: [f64; 30] = [
    -4.41534164647933937950e-18,
    3.33079451882223809783e-17,
    -2.43127984654795469359e-16,
    1.71539128555513303061e-15,
    -1.16853328779934516808e-14,
    7.67618549860493561688e-14,
    -4.85644678311192946090e-13,
    2.95505266312963983461e-12,
    -1.72682629144155570723e-11,
    9.67580903537323691224e-11,
    -5.18979560163526290666e-10,
    2.65982372468238665035e-9,
    -1.30002500998624804212e-8,
    6.04699502254191894932e-8,
    -2.67079385394061173391e-7,
    1.11738753912010371815e-6,
    -4.41673835845875056359e-6,
    1.64484480707288970893e-5,
    -5.75419501008210370398e-5,
    1.88502885095841655729e-4,
    -5.76375574538582365885e-4,
    1.63947561694133579842e-3,
    -4.32430999505057594430e-3,
    1.05464603945949983183e-2,
    -2.37374148058994688156e-2,
    4.93052842396707084878e-2,
    -9.49010970480476444210e-2,
    1.71620901522208775349e-1,
    -3.04682672343198398683e-1,
    6.76795274409476084995e-1,
];

/// Chebyshev coefficients for $\exp(-x) \sqrt{x} I_0(x)$ in the inverted interval $[8, \infty)$.
const B_I0: [f64; 25] = [
    -7.23318048787475395456e-18,
    -4.83050448594418207126e-18,
    4.46562142029675999901e-17,
    3.46122286769746109310e-17,
    -2.82762398051658348494e-16,
    -3.42548561967721913462e-16,
    1.77256013305652638360e-15,
    3.81168066935262242075e-15,
    -9.55484669882830764870e-15,
    -4.15056934728722208663e-14,
    1.54008621752140982691e-14,
    3.85277838274214270114e-13,
    7.18012445138366623367e-13,
    -1.79417853150680611778e-12,
    -1.32158118404477131188e-11,
    -3.14991652796324136454e-11,
    1.18891471078464383424e-11,
    4.94060238822496958910e-10,
    3.39623202570838634515e-9,
    2.26666899049817806459e-8,
    2.04891858946906374183e-7,
    2.89137052083475648297e-6,
    6.88975834691682398426e-5,
    3.36911647825569408990e-3,
    8.04490411014108831608e-1,
];

/// Evaluates a Chebyshev polynomial series using Clenshaw's recurrence algorithm.
#[inline]
fn evaluate_chebyshev(x: f64, coefficients: &[f64]) -> f64 {
    let mut p = 0;
    let mut b0 = coefficients[p];
    p += 1;
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    let mut i = coefficients.len() - 1;

    while i > 0 {
        b2 = b1;
        b1 = b0;
        b0 = x * b1 - b2 + coefficients[p];
        p += 1;
        i -= 1;
    }

    0.5 * (b0 - b2)
}

/// Computes the zero-order modified Bessel function of the first kind, $I_0(x)$.
pub fn modified_bessel_i0(mut x: f64) -> f64 {
    if x < 0.0 {
        x = -x;
    }

    if x <= 8.0 {
        let y = (x / 2.0) - 2.0;
        x.exp() * evaluate_chebyshev(y, &I0_COEFFICIENTS)
    } else {
        x.exp() * evaluate_chebyshev(32.0 / x - 2.0, &B_I0) / x.sqrt()
    }
}

/// Generates a Kaiser window of specified length and shape parameter $\beta$.
///
/// $$w[n] = \frac{I_0\left(\beta \sqrt{1 - \left(\frac{2n}{N-1} - 1\right)^2}\right)}{I_0(\beta)}$$
pub fn kaiser_window(samples: usize, beta: f64) -> Vec<f64> {
    assert!(samples > 1, "Window length must be > 1");
    let mut result = Vec::with_capacity(samples);
    let m = (samples - 1) as f64;
    let den = modified_bessel_i0(beta);

    for i in 0..samples {
        let arg = 2.0 * (i as f64) / m - 1.0;
        let rad = (1.0 - arg * arg).max(0.0).sqrt();
        let num = modified_bessel_i0(beta * rad);
        result.push(num / den);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
