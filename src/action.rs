//! Everything the user can ask the explorer to do.

use crate::fractal::palette::Palette;
use crate::math::Complex;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    /// Drag the view by a pixel delta.
    Pan {
        dx: f64,
        dy: f64,
    },
    /// Zoom about a pixel position, keeping the world point under it fixed.
    ZoomAt {
        px: f64,
        py: f64,
        factor: f64,
    },
    /// Jump the centre to an absolute point.
    SetCenter(Complex),
    /// Multiply the iteration bias
    ScaleIterBias(f64),
    /// Set the iteration bias
    SetIterBias(f64),
    SetPalette(Palette),
    /// Advance to the next palette
    CyclePalette,
    /// Return to the home view
    ResetView,
    /// Print the current coordinates
    LogPosition,
    /// Quit
    Exit,
}

impl Action {
    /// Whether this action is Apps or Scenes.
    pub fn is_app_level(self) -> bool {
        matches!(self, Action::Exit)
    }
}
