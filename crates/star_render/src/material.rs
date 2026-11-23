//! How the surface of a 3D mesh looks.

use crate::Color;

/// The look of a 3D mesh's surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// The color of the surface, in sRGB.
    pub base_color: Color,
}

impl Material {
    /// A surface of `base_color`.
    #[must_use]
    pub const fn new(base_color: Color) -> Self {
        Self { base_color }
    }
}

impl Default for Material {
    /// A white surface.
    fn default() -> Self {
        Self::new(Color::WHITE)
    }
}
