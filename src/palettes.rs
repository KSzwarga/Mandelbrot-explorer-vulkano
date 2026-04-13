// ─────────────────────────────────────────────────────────────────────────────
// palettes.rs
//
// All available colour palettes. Each variant corresponds to a separately
// compiled fragment shader module in pipeline.rs.
//
// Switching palette at runtime calls create_pipeline() with the new variant,
//
// To add a new palette:
//   1. Add a variant here.
//   2. Add a mod fs_<name> { vulkano_shaders::shader! { ... } } in pipeline.rs.
//   3. Add the match arm in create_pipeline().
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// Warm red/orange fills, violet background, muted/serious.
    WarmViolet,
    /// Classic blue→cyan→yellow→red high-contrast bands.
    BlueFire,
    /// Deep blacks, electric blue filaments, gold halos.
    Midnight,
    /// Soft Acids: pink, lavender, mint — gentle on the eyes.
    Acid,
    /// Ultra-high contrast greyscale with bright white boundary detail.
    Greyscale,
}

impl PaletteMode {
    /// All variants in display order (used for cycling with Tab / number keys).
    pub const ALL: &'static [PaletteMode] = &[
        PaletteMode::WarmViolet,
        PaletteMode::BlueFire,
        PaletteMode::Midnight,
        PaletteMode::Acid,
        PaletteMode::Greyscale,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaletteMode::WarmViolet => "Warm Violet",
            PaletteMode::BlueFire   => "Blue Fire",
            PaletteMode::Midnight   => "Midnight",
            PaletteMode::Acid     => "Acid",
            PaletteMode::Greyscale  => "Greyscale",
        }
    }

    /// Cycle to the next palette, wrapping around.
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Cycle to the previous palette, wrapping around.
    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}
