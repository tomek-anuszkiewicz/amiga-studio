//! Direct Form digital biquad filter implementation.
//!
//! Converts continuous $s$-domain transfer functions to the discrete $z$-domain
//! via the Bilinear Transform:
//! $$s \leftarrow \frac{2}{T} \frac{1 - z^{-1}}{1 + z^{-1}}$$

/// Direct Form digital biquad filter.
///
/// Implements difference equation:
/// $$y[n] = b_0 x[n] + b_1 x[n-1] + b_2 x[n-2] - a_1 y[n-1] - a_2 y[n-2]$$
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    /// Creates a digital biquad filter from continuous transfer function denominator coefficients:
    ///
    /// $$H(s) = \frac{1}{\text{den\_s2} \cdot s^2 + \text{den\_s1} \cdot s + 1}$$
    ///
    /// If `den_s2 == 0`, a 1st-order RC filter is modeled.
    pub fn from_continuous_transfer_function(den_s2: f64, den_s1: f64, sampling_rate: f64) -> Self {
        let t = 1.0 / sampling_rate;

        if den_s2 == 0.0 {
            let rc = den_s1;
            let b0 = t / (rc + t);
            let a1 = b0 - 1.0;
            Self {
                b0,
                b1: 0.0,
                b2: 0.0,
                a1,
                a2: 0.0,
                x1: 0.0,
                x2: 0.0,
                y1: 0.0,
                y2: 0.0,
            }
        } else {
            let a = den_s2;
            let b = den_s1;
            let a0 = 4.0 * a + 2.0 * b * t + t * t;
            let b0 = t * t / a0;
            let b1 = 2.0 * t * t / a0;
            let b2 = t * t / a0;
            let a1 = (2.0 * t * t - 8.0 * a) / a0;
            let a2 = (4.0 * a - 2.0 * b * t + t * t) / a0;
            Self {
                b0,
                b1,
                b2,
                a1,
                a2,
                x1: 0.0,
                x2: 0.0,
                y1: 0.0,
                y2: 0.0,
            }
        }
    }

    /// Resets internal delay line memory registers to zero.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Processes a single input sample through the biquad filter.
    #[inline]
    pub fn filter_sample(&mut self, x0: f64) -> f64 {
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }

    /// Warms up the filter with `sample` for 10,000 cycles to settle delay lines into DC steady state.
    pub fn settle(&mut self, sample: f64) {
        for _ in 0..10_000 {
            self.filter_sample(sample);
        }
    }

    /// Filters an in-place array of signal samples, settling DC state beforehand.
    pub fn filter_signal(&mut self, signal: &mut [f64]) {
        self.reset();
        if let Some(&first) = signal.first() {
            self.settle(first);
        }
        for sample in signal.iter_mut() {
            *sample = self.filter_sample(*sample);
        }
    }
}
