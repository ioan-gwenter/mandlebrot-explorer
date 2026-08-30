//! The user-editable state of the explorer, and the one place it is mutated.

use crate::action::Action;
use crate::fractal::palette::Palette;
use crate::fractal::reference::Reference;
use crate::fractal::view::View;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[must_use]
pub enum Redraw {
    Yes,
    No,
}

impl Redraw {
    pub fn is_needed(self) -> bool {
        self == Redraw::Yes
    }

    fn from_changed(changed: bool) -> Self {
        if changed { Redraw::Yes } else { Redraw::No }
    }
}

pub struct Scene {
    pub view: View,
    pub palette: Palette,
    reference: Reference,
    /// Bumped whenever `reference` is replaced, so the renderer can tell
    /// whether its uploaded copy is still current.
    reference_generation: u64,
}

impl Scene {
    pub fn new(width: u32, height: u32) -> Self {
        let view = View::new(width, height);
        let reference = Reference::compute(view.center, view.max_iter());
        Self {
            view,
            palette: Palette::default(),
            reference,
            reference_generation: 0,
        }
    }

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn reference_generation(&self) -> u64 {
        self.reference_generation
    }

    pub fn sync_reference(&mut self) {
        let max_iter = self.view.max_iter();
        let half_extent = self.view.half_extent();

        if !self
            .reference
            .is_valid_for(self.view.center, max_iter, half_extent)
        {
            self.reference = Reference::compute(self.view.center, max_iter);
            self.reference_generation += 1;
        }
    }

    /// Apply one action. The single mutation point for scene state.
    pub fn apply(&mut self, action: Action) -> Redraw {
        match action {
            Action::Pan { dx, dy } => {
                self.view.pan_pixels(dx, dy);
                Redraw::Yes
            }
            Action::ZoomAt { px, py, factor } => {
                self.view.zoom_at(px, py, factor);
                Redraw::Yes
            }

            Action::SetCenter(c) => Redraw::from_changed(self.view.set_center(c)),
            Action::ScaleIterBias(f) => {
                let before = self.view.iter_bias;
                self.view.set_iter_bias(before * f);
                log::info!(
                    "iter_bias {:.2} -> max_iter {}",
                    self.view.iter_bias,
                    self.view.max_iter()
                );
                Redraw::from_changed(self.view.iter_bias != before)
            }
            Action::SetIterBias(bias) => {
                let before = self.view.iter_bias;
                self.view.set_iter_bias(bias);
                Redraw::from_changed(self.view.iter_bias != before)
            }
            Action::SetPalette(p) => {
                let changed = self.palette != p;
                self.palette = p;
                Redraw::from_changed(changed)
            }
            Action::CyclePalette => {
                self.palette = self.palette.next();
                log::info!("palette: {}", self.palette.name());
                Redraw::Yes
            }
            Action::ResetView => {
                self.view.reset();
                Redraw::Yes
            }
            Action::LogPosition => {
                log::info!(
                    "center = {:.17} {:+.17}i | zoom = {:.3e}x | iter = {}",
                    self.view.center.re,
                    self.view.center.im,
                    self.view.zoom_level(),
                    self.view.max_iter(),
                );
                Redraw::No
            }
            // App owns the event loop, so it handles this before dispatching.
            Action::Exit => Redraw::No,
        }
    }

    /// Track a window resize, keeping the view's fit reference current.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.view.resize(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Complex;

    fn scene() -> Scene {
        Scene::new(800, 600)
    }

    #[test]
    fn zoom_keeps_the_cursor_point_fixed() {
        let mut s = scene();
        let before = s.view.screen_to_world(200.0, 150.0);
        let _ = s.apply(Action::ZoomAt {
            px: 200.0,
            py: 150.0,
            factor: 0.5,
        });
        let after = s.view.screen_to_world(200.0, 150.0);
        assert!((before.re - after.re).abs() < 1e-12);
        assert!((before.im - after.im).abs() < 1e-12);
    }

    #[test]
    fn iter_bias_clamps_through_repeated_scaling() {
        let mut s = scene();
        for _ in 0..100 {
            let _ = s.apply(Action::ScaleIterBias(1.5));
        }
        assert_eq!(s.view.iter_bias, *View::ITER_BIAS_RANGE.end());
        for _ in 0..100 {
            let _ = s.apply(Action::ScaleIterBias(1.0 / 1.5));
        }
        assert_eq!(s.view.iter_bias, *View::ITER_BIAS_RANGE.start());
    }

    /// The keyboard shortcut and the HUD slider must land in the same place.
    #[test]
    fn scaling_and_setting_iter_bias_agree_at_the_bound() {
        let mut a = scene();
        let mut b = scene();
        for _ in 0..100 {
            let _ = a.apply(Action::ScaleIterBias(1.5));
        }
        let _ = b.apply(Action::SetIterBias(1e9));
        assert_eq!(a.view.iter_bias, b.view.iter_bias);
    }

    #[test]
    fn palette_cycles_and_wraps() {
        let mut s = scene();
        let start = s.palette;
        for _ in 0..Palette::ALL.len() {
            let _ = s.apply(Action::CyclePalette);
        }
        assert_eq!(s.palette, start);
    }

    #[test]
    fn setting_the_same_palette_needs_no_redraw() {
        let mut s = scene();
        let current = s.palette;
        assert_eq!(s.apply(Action::SetPalette(current)), Redraw::No);
        assert_eq!(s.apply(Action::SetPalette(current.next())), Redraw::Yes);
    }

    #[test]
    fn logging_position_never_forces_a_redraw() {
        assert_eq!(scene().apply(Action::LogPosition), Redraw::No);
    }

    /// Both reset paths are this one action, so they cannot disagree.
    #[test]
    fn reset_after_resize_fits_the_new_window() {
        let mut s = scene();
        s.resize(1024, 768);
        let _ = s.apply(Action::ZoomAt {
            px: 10.0,
            py: 10.0,
            factor: 0.1,
        });
        let _ = s.apply(Action::ResetView);

        let fresh = Scene::new(1024, 768);
        assert!((s.view.scale - fresh.view.scale).abs() < 1e-18);
        assert_eq!(s.view.center, fresh.view.center);
    }

    #[test]
    fn panning_a_little_reuses_the_reference_orbit() {
        let mut s = scene();
        s.sync_reference();
        let generation = s.reference_generation();

        for _ in 0..30 {
            let _ = s.apply(Action::Pan { dx: 1.0, dy: 1.0 });
            s.sync_reference();
        }

        assert_eq!(
            s.reference_generation(),
            generation,
            "small pans must not regenerate the reference orbit"
        );
    }

    #[test]
    fn jumping_far_regenerates_the_reference_orbit() {
        let mut s = scene();
        s.sync_reference();
        let generation = s.reference_generation();

        let _ = s.apply(Action::SetCenter(Complex::new(10.0, 10.0)));
        s.sync_reference();

        assert!(s.reference_generation() > generation);
    }

    /// A non-finite centre used to be accepted straight from the HUD, and it
    /// hung the explorer: NaN fails every comparison, so `is_valid_for` was
    /// false forever and the orbit never escaped its bailout — a full
    /// iteration budget recomputed on the CPU on every single frame.
    #[test]
    fn a_non_finite_center_is_refused_and_leaves_the_orbit_alone() {
        let mut s = scene();
        s.sync_reference();
        let generation = s.reference_generation();
        let center = s.view.center;

        for bad in [
            Complex::new(f64::NAN, 0.0),
            Complex::new(0.0, f64::NAN),
            Complex::new(f64::INFINITY, 0.0),
            Complex::new(0.0, f64::NEG_INFINITY),
        ] {
            assert_eq!(s.apply(Action::SetCenter(bad)), Redraw::No, "{bad:?}");
            s.sync_reference();
        }

        assert_eq!(s.view.center, center);
        assert_eq!(
            s.reference_generation(),
            generation,
            "a refused jump must not rebuild the reference orbit"
        );
    }

    #[test]
    fn sync_is_idempotent_when_nothing_moves() {
        let mut s = scene();
        s.sync_reference();
        let generation = s.reference_generation();
        for _ in 0..10 {
            s.sync_reference();
        }
        assert_eq!(s.reference_generation(), generation);
    }
}
