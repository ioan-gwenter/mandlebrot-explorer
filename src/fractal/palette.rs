//! Colour schemes for the escape-time gradient.
//!
//! The discriminant is passed to the shader as a `u32` and selected in
//! `palette()` in `shaders/mandelbrot.wgsl`, so the variant *order* here is
//! part of that contract — adding a variant means adding the matching arm
//! there. Everything on the Rust side (cycling, names, the HUD combo box) is
//! driven from [`Palette::ALL`], so this enum and the shader are the only two
//! places to touch.

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Palette {
    #[default]
    Rainbow,
    Ember,
    Ocean,
    Grey,
}

impl Palette {
    /// Every variant, in shader-index order. The single list the rest of the
    /// crate iterates.
    pub const ALL: [Palette; 4] = [
        Palette::Rainbow,
        Palette::Ember,
        Palette::Ocean,
        Palette::Grey,
    ];

    /// The next palette, wrapping. Drives the `P` shortcut.
    pub fn next(self) -> Self {
        Self::ALL[(self.index() as usize + 1) % Self::ALL.len()]
    }

    /// Index handed to the shader; must match the `switch` in the WGSL.
    pub fn index(self) -> u32 {
        self as u32
    }

    pub fn name(self) -> &'static str {
        match self {
            Palette::Rainbow => "rainbow",
            Palette::Ember => "ember",
            Palette::Ocean => "ocean",
            Palette::Grey => "grey",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_is_in_index_order() {
        for (i, p) in Palette::ALL.iter().enumerate() {
            assert_eq!(p.index() as usize, i, "{p:?} is out of order in ALL");
        }
    }

    #[test]
    fn next_cycles_through_every_variant_and_wraps() {
        let mut seen = Vec::new();
        let mut p = Palette::default();
        for _ in 0..Palette::ALL.len() {
            seen.push(p);
            p = p.next();
        }
        assert_eq!(p, Palette::default(), "should wrap to the start");
        assert_eq!(seen, Palette::ALL);
    }

    #[test]
    fn names_are_distinct() {
        let mut names: Vec<_> = Palette::ALL.iter().map(|p| p.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "palette names must be unique");
    }
}
