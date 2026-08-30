//! The camera: where we are on the complex plane and how far in we are zoomed.

use crate::math::Complex;

/// Where the explorer starts, and where [`View::reset`] returns to. Offset from
/// the origin so the whole set sits centred in frame.
const HOME_CENTER: Complex = Complex::new(-0.5, 0.0);

/// Complex-plane extents the initial view fits into the window.
const FIT_WIDTH: f64 = 3.5;
const FIT_HEIGHT: f64 = 2.4;

/// Floor on world units per pixel. Past roughly this point the `f32` deltas the
/// shader works in stop resolving detail, so zooming further only blurs.
const MIN_SCALE: f64 = 1e-15;
const MAX_SCALE: f64 = 1.0;

/// Iteration budget as a function of zoom: deeper views need more iterations to
/// separate the interior from the boundary. `ITER_PER_OCTAVE` is added per
/// doubling of magnification.
const ITER_BASE: f64 = 100.0;
const ITER_PER_OCTAVE: f64 = 120.0;
const ITER_MIN: f64 = 64.0;
const ITER_MAX: f64 = 50_000.0;

pub struct View {
    pub center: Complex,
    /// World units per pixel
    pub scale: f64,
    pub iter_bias: f64,
    /// Scale at which the set exactly fits the current window
    base_scale: f64,
    width: u32,
    height: u32,
}

impl View {
    pub const ITER_BIAS_RANGE: std::ops::RangeInclusive<f64> = 0.1..=20.0;

    pub fn new(width: u32, height: u32) -> Self {
        let base_scale = Self::fit_scale(width, height);
        Self {
            center: HOME_CENTER,
            scale: base_scale,
            iter_bias: 1.0,
            base_scale,
            width,
            height,
        }
    }

    /// Scale at which [`FIT_WIDTH`] x [`FIT_HEIGHT`] fits inside the window.
    fn fit_scale(width: u32, height: u32) -> f64 {
        let sx = FIT_WIDTH / width.max(1) as f64;
        let sy = FIT_HEIGHT / height.max(1) as f64;
        sx.max(sy)
    }

    /// Track a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.base_scale = Self::fit_scale(width, height);
    }

    pub fn reset(&mut self) {
        self.center = HOME_CENTER;
        self.scale = self.base_scale;
        self.iter_bias = 1.0;
    }

    pub fn screen_to_world(&self, px: f64, py: f64) -> Complex {
        let w = self.width as f64;
        let h = self.height as f64;
        Complex::new(
            self.center.re + (px - w * 0.5) * self.scale,
            // flip y: screen down is negative imaginary
            self.center.im + (h * 0.5 - py) * self.scale,
        )
    }

    /// Zoom by `factor` while keeping the world point under the cursor fixed.
    pub fn zoom_at(&mut self, px: f64, py: f64, factor: f64) {
        let before = self.screen_to_world(px, py);
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let after = self.screen_to_world(px, py);
        self.center = self.center + (before - after);
    }

    pub fn pan_pixels(&mut self, dx: f64, dy: f64) {
        self.center.re -= dx * self.scale;
        self.center.im += dy * self.scale;
    }

    pub fn set_center(&mut self, center: Complex) -> bool {
        if !center.is_finite() || center == self.center {
            return false;
        }
        self.center = center;
        true
    }

    /// Clamp `bias` into [`View::ITER_BIAS_RANGE`] and store it.

    pub fn set_iter_bias(&mut self, bias: f64) {
        if bias.is_nan() {
            return;
        }
        self.iter_bias = bias.clamp(*Self::ITER_BIAS_RANGE.start(), *Self::ITER_BIAS_RANGE.end());
    }

    /// Magnification relative to the initial fitted view.
    pub fn zoom_level(&self) -> f64 {
        self.base_scale / self.scale
    }

    pub fn max_iter(&self) -> u32 {
        let zoom = self.zoom_level().max(1.0);
        let n = (ITER_BASE + ITER_PER_OCTAVE * zoom.log2()) * self.iter_bias;
        n.clamp(ITER_MIN, ITER_MAX) as u32
    }

    /// Half the diagonal of the visible region, in world units. 
    pub fn half_extent(&self) -> f64 {
        let w = self.width as f64 * self.scale;
        let h = self.height as f64 * self.scale;
        0.5 * (w * w + h * h).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_cursor_point_fixed() {
        let mut v = View::new(800, 600);
        let before = v.screen_to_world(200.0, 150.0);
        v.zoom_at(200.0, 150.0, 0.5);
        let after = v.screen_to_world(200.0, 150.0);
        assert!((before.re - after.re).abs() < 1e-12);
        assert!((before.im - after.im).abs() < 1e-12);
    }

    #[test]
    fn fresh_view_sits_at_zoom_one() {
        assert!((View::new(800, 600).zoom_level() - 1.0).abs() < 1e-12);
    }

    /// Regression: `base_scale` used to be captured once in `new` and never
    /// updated, so after a resize `zoom_level` divided by a stale reference.
    #[test]
    fn resize_keeps_zoom_level_honest() {
        let mut v = View::new(800, 600);
        v.resize(1600, 1200);
        v.reset();
        assert!((v.zoom_level() - 1.0).abs() < 1e-12);
    }

    /// Regression: the HUD reset button restored the old `base_scale`, so it
    /// disagreed with the R key, which rebuilt the view at current dimensions.
    #[test]
    fn reset_after_resize_matches_a_fresh_view() {
        let mut v = View::new(800, 600);
        v.resize(1024, 768);
        v.zoom_at(10.0, 10.0, 0.1);
        v.reset();

        let fresh = View::new(1024, 768);
        assert!((v.scale - fresh.scale).abs() < 1e-18);
        assert_eq!(v.center, fresh.center);
        assert_eq!(v.max_iter(), fresh.max_iter());
    }

    #[test]
    fn iter_bias_clamps_at_both_ends() {
        let mut v = View::new(800, 600);
        v.set_iter_bias(1e6);
        assert_eq!(v.iter_bias, *View::ITER_BIAS_RANGE.end());
        v.set_iter_bias(-5.0);
        assert_eq!(v.iter_bias, *View::ITER_BIAS_RANGE.start());
        v.set_iter_bias(f64::INFINITY);
        assert_eq!(v.iter_bias, *View::ITER_BIAS_RANGE.end());
        v.set_iter_bias(f64::NEG_INFINITY);
        assert_eq!(v.iter_bias, *View::ITER_BIAS_RANGE.start());
    }

    /// `f64::clamp` returns NaN unchanged, so this needs its own guard.
    #[test]
    fn a_nan_iter_bias_is_dropped_not_clamped() {
        let mut v = View::new(800, 600);
        v.set_iter_bias(f64::NAN);
        assert_eq!(v.iter_bias, 1.0);
        assert!(v.max_iter() >= 64, "a NaN bias must not zero the budget");
    }

    #[test]
    fn set_center_refuses_a_non_finite_point() {
        let mut v = View::new(800, 600);
        let home = v.center;

        for bad in [
            Complex::new(f64::NAN, 0.0),
            Complex::new(0.0, f64::NAN),
            Complex::new(f64::INFINITY, 0.0),
            Complex::new(0.0, f64::NEG_INFINITY),
        ] {
            assert!(!v.set_center(bad), "{bad:?} should be refused");
            assert_eq!(v.center, home, "a refused jump must not move the camera");
        }
    }

    #[test]
    fn set_center_reports_whether_it_moved() {
        let mut v = View::new(800, 600);
        let target = Complex::new(0.25, -0.5);
        assert!(v.set_center(target));
        assert_eq!(v.center, target);
        assert!(!v.set_center(target), "an unchanged centre is not a move");
    }

    #[test]
    fn zooming_in_raises_the_iteration_budget() {
        let mut v = View::new(800, 600);
        let shallow = v.max_iter();
        for _ in 0..40 {
            v.zoom_at(400.0, 300.0, 0.5);
        }
        assert!(v.max_iter() > shallow);
    }

    #[test]
    fn scale_never_escapes_its_bounds() {
        let mut v = View::new(800, 600);
        for _ in 0..500 {
            v.zoom_at(400.0, 300.0, 0.5);
        }
        assert!(v.scale >= MIN_SCALE);
        for _ in 0..500 {
            v.zoom_at(400.0, 300.0, 2.0);
        }
        assert!(v.scale <= MAX_SCALE);
    }
}
