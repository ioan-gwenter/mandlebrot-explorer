//! The reference orbit behind perturbation rendering.
//!
//! Iterating z -> z^2 + c in `f32` runs out of precision long before the
//! interesting zoom depths. Instead we iterate one point in `f64` on the CPU —
//! the reference — and let the shader track each pixel's small *delta* from it
//! in `f32`. One reference serves a whole neighbourhood of pixels, so it only
//! needs regenerating when the view leaves that neighbourhood.

use crate::math::Complex;

/// Hard cap on stored orbit points, bounding both the CPU work and the storage
/// buffer at `MAX_REF * 8` bytes.
pub const MAX_REF: usize = 100_000;

/// Escape threshold
const BAILOUT: f64 = 65536.0;

pub struct Reference {
    pub center: Complex,
    pub orbit: Vec<[f32; 2]>,
    /// Whether the orbit terminated by escaping rather than by running out of
    /// iterations. An escaped orbit is complete: it stays valid no matter how
    /// far `max_iter` later rises.
    pub escaped: bool,
}

impl Reference {
    pub fn compute(center: Complex, max_iter: u32) -> Self {
        let cap = (max_iter as usize + 1).min(MAX_REF);
        let mut orbit = Vec::with_capacity(cap);
        let mut z = Complex::ZERO;
        let mut escaped = false;

        for _ in 0..cap {
            orbit.push(z.to_f32_pair());

            // Check before stepping so the escaping value itself lands in the orbit.
            if z.norm_sqr() > BAILOUT {
                escaped = true;
                break;
            }

            z = z * z + center;
        }

        Self {
            center,
            orbit,
            escaped,
        }
    }

    /// Whether this orbit can still serve a view at `center` with `max_iter`.
    ///
    /// Two independent conditions:
    ///
    /// 1. **Near enough.** The reference must lie within the region being drawn
    ///    for the deltas to stay small. `half_extent` is the view's half
    ///    diagonal; [`REUSE_FRACTION`] keeps the reference comfortably inside
    ///    it rather than out at the very edge.
    /// 2. **Long enough.** The orbit must cover the requested iteration count —
    ///    unless it escaped, in which case it is already complete.
    pub fn is_valid_for(&self, center: Complex, max_iter: u32, half_extent: f64) -> bool {
        let near_enough = (center - self.center).norm() <= half_extent * REUSE_FRACTION;
        let long_enough = self.covers(max_iter);
        near_enough && long_enough
    }

    /// Whether the orbit runs far enough for `max_iter` iterations. A shorter
    /// orbit that escaped is still complete; the shader stops at the escape.
    fn covers(&self, max_iter: u32) -> bool {
        self.escaped || self.orbit.len() >= (max_iter as usize).min(MAX_REF)
    }
}

/// How far the view centre may drift from the reference, as a fraction of the
/// view's half diagonal, before the orbit is regenerated.
const REUSE_FRACTION: f64 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;

    /// A centre well inside the set, so its orbit never escapes.
    const DEEP: Complex = Complex::new(-0.743_643_887_037_151, 0.131_825_904_205_33);

    #[test]
    fn orbit_matches_direct_iteration() {
        let r = Reference::compute(DEEP, 64);

        let mut z = Complex::ZERO;
        for point in &r.orbit {
            assert!((point[0] as f64 - z.re).abs() < 1e-6, "re mismatch");
            assert!((point[1] as f64 - z.im).abs() < 1e-6, "im mismatch");
            z = z * z + DEEP;
        }
    }

    #[test]
    fn escaping_reference_terminates_early() {
        // (1, 1) leaves the set almost immediately.
        let r = Reference::compute(Complex::new(1.0, 1.0), 10_000);
        assert!(r.orbit.len() < 20);
        assert!(r.escaped);
    }

    #[test]
    fn a_small_nudge_reuses_the_orbit() {
        let r = Reference::compute(DEEP, 500);
        let nudged = DEEP + Complex::new(1e-9, 1e-9);
        assert!(r.is_valid_for(nudged, 500, 1e-6));
    }

    #[test]
    fn a_large_jump_regenerates() {
        let r = Reference::compute(DEEP, 500);
        let far = DEEP + Complex::new(1.0, 0.0);
        assert!(!r.is_valid_for(far, 500, 1e-6));
    }

    #[test]
    fn lowering_max_iter_never_regenerates() {
        let r = Reference::compute(DEEP, 5_000);
        assert!(r.is_valid_for(DEEP, 100, 1.0));
    }

    #[test]
    fn raising_max_iter_past_the_orbit_regenerates() {
        let r = Reference::compute(DEEP, 500);
        assert!(!r.escaped, "this centre should not escape");
        assert!(!r.is_valid_for(DEEP, 5_000, 1.0));
    }

    /// An escaped orbit is complete, so a higher `max_iter` must not force a
    /// pointless recompute.
    #[test]
    fn an_escaped_orbit_stays_valid_at_higher_max_iter() {
        let r = Reference::compute(Complex::new(1.0, 1.0), 100);
        assert!(r.escaped);
        assert!(r.is_valid_for(Complex::new(1.0, 1.0), 50_000, 1.0));
    }

    #[test]
    fn orbit_is_capped_at_max_ref() {
        let r = Reference::compute(DEEP, u32::MAX);
        assert!(r.orbit.len() <= MAX_REF);
    }
}
