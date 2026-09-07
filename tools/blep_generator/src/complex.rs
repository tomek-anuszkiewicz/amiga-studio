use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A double-precision 64-bit complex number.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };
    #[allow(dead_code)]
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    #[inline]
    pub const fn from_real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    /// Complex conjugate ($a - bi$).
    #[inline]
    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    /// Euclidean magnitude / absolute value ($|z| = \sqrt{a^2 + b^2}$).
    #[inline]
    pub fn magnitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// Squared magnitude ($a^2 + b^2$).
    #[allow(dead_code)]
    #[inline]
    pub fn magnitude_squared(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Complex exponential ($e^z = e^a (\cos b + i \sin b)$).
    #[inline]
    pub fn exp(self) -> Self {
        let r = self.re.exp();
        Self {
            re: r * self.im.cos(),
            im: r * self.im.sin(),
        }
    }
}

impl Add for Complex64 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl AddAssign for Complex64 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
}

impl Sub for Complex64 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl SubAssign for Complex64 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.re -= rhs.re;
        self.im -= rhs.im;
    }
}

impl Mul for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl Mul<f64> for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self {
            re: self.re * rhs,
            im: self.im * rhs,
        }
    }
}

impl MulAssign<f64> for Complex64 {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.re *= rhs;
        self.im *= rhs;
    }
}

impl Div<f64> for Complex64 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self {
        Self {
            re: self.re / rhs,
            im: self.im / rhs,
        }
    }
}

impl DivAssign<f64> for Complex64 {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.re /= rhs;
        self.im /= rhs;
    }
}

impl Neg for Complex64 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
