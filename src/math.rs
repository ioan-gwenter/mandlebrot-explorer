//! Minimal complex arithmetic.

use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Squared magnitude. Avoids the square root when only comparing against
    /// threshold
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn norm(self) -> f64 {
        self.norm_sqr().sqrt()
    }

    /// Whether both parts are ordinary numbers
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// Narrow to the `vec2<f32>` pair the shader's storage buffer expects.
    pub fn to_f32_pair(self) -> [f32; 2] {
        [self.re as f32, self.im as f32]
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplication_matches_the_hand_expansion() {
        let a = Complex::new(3.0, -2.0);
        let b = Complex::new(-1.0, 4.0);
        let p = a * b;
        assert_eq!(p.re, -3.0 - (-8.0)); // ac - bd
        assert_eq!(p.im, 12.0 + 2.0); // ad + bc
    }

    #[test]
    fn norm_sqr_avoids_the_root() {
        let z = Complex::new(3.0, 4.0);
        assert_eq!(z.norm_sqr(), 25.0);
        assert_eq!(z.norm(), 5.0);
    }

    #[test]
    fn is_finite_rejects_a_bad_part_on_either_side() {
        assert!(Complex::new(-0.75, 0.1).is_finite());
        assert!(!Complex::new(f64::NAN, 0.1).is_finite());
        assert!(!Complex::new(0.1, f64::NAN).is_finite());
        assert!(!Complex::new(f64::INFINITY, 0.1).is_finite());
        assert!(!Complex::new(0.1, f64::NEG_INFINITY).is_finite());
    }
}
