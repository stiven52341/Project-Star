//! How the surface of a 3D mesh looks, in the metallic-roughness model
//! that glTF and most engines use.

use crate::{Color, TextureId};

/// Roughness of a new material: neither a mirror nor chalk.
pub const DEFAULT_ROUGHNESS: f32 = 0.5;

/// The look of a 3D mesh's surface. Values outside `0..=1` are clamped
/// when drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// The color of the surface, in sRGB: what diffuses light, or the tint
    /// of the reflections of a metal.
    pub base_color: Color,
    /// An image multiplied by `base_color`, placed by the mesh's texture
    /// coordinates.
    pub base_color_texture: Option<TextureId>,
    /// 0 for plastic, wood, stone…; 1 for metals, which reflect in their
    /// own color and diffuse nothing.
    pub metallic: f32,
    /// 0 for a mirror-like surface with sharp highlights, 1 for a matte one
    /// that spreads them out.
    pub roughness: f32,
}

impl Material {
    /// A surface of `base_color`, not metallic, with [`DEFAULT_ROUGHNESS`].
    #[must_use]
    pub const fn new(base_color: Color) -> Self {
        Self {
            base_color,
            base_color_texture: None,
            metallic: 0.0,
            roughness: DEFAULT_ROUGHNESS,
        }
    }

    /// The same material, showing `texture` tinted by its base color.
    #[must_use]
    pub const fn with_texture(mut self, texture: TextureId) -> Self {
        self.base_color_texture = Some(texture);
        self
    }

    /// The same material, with `metallic`.
    #[must_use]
    pub const fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    /// The same material, with `roughness`.
    #[must_use]
    pub const fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }
}

impl Default for Material {
    /// A white surface.
    fn default() -> Self {
        Self::new(Color::WHITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_change_one_value_each() {
        let material = Material::new(Color::BLACK)
            .with_texture(TextureId(3))
            .with_metallic(1.0)
            .with_roughness(0.2);

        assert_eq!(material.base_color, Color::BLACK);
        assert_eq!(material.base_color_texture, Some(TextureId(3)));
        assert_eq!((material.metallic, material.roughness), (1.0, 0.2));
        assert_eq!(Material::default().roughness, DEFAULT_ROUGHNESS);
    }
}
