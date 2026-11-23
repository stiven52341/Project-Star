//! Meshes of basic shapes, built by the engine when needed.

use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};
use star_math::{Vec2, Vec3};
use star_render::{Color, DEFAULT_ROUGHNESS, Material, MeshShape, TextureFilter, TextureId};

use crate::texture_slot::TextureSlot;

/// The radius of the sphere every sphere is drawn from, scaled.
const UNIT_SPHERE_RADIUS: f32 = 0.5;

/// Shows a mesh of a basic shape at a 3D node, like a
/// [`MeshNode`](star_scene::MeshNode), but named by its shape and its image
/// file: the engine builds the mesh and loads the texture the first time it
/// draws them, and again whenever they change. So code without access to
/// the renderer, such as StarScript games, can give nodes meshes.
///
/// Every shape of a kind is drawn from one mesh of size 1, scaled: changing
/// the sizes every frame builds nothing new.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshFile {
    /// The shape, with its sizes.
    pub shape: MeshShape,
    /// The color of the surface, in sRGB.
    pub color: Color,
    /// How metallic the surface is, from 0 to 1.
    pub metallic: f32,
    /// How rough the surface is, from 0 to 1.
    pub roughness: f32,
    /// The image multiplied by the color, in the asset folder.
    pub texture: Option<AssetPath>,
    /// How the image is sampled.
    pub filter: TextureFilter,
    loaded: TextureSlot,
    /// The last shape that could not be drawn, so it is reported once.
    rejected: Option<MeshShape>,
}

impl MeshFile {
    /// A white mesh of `shape`, not metallic, with the default roughness
    /// and no image.
    #[must_use]
    pub const fn new(shape: MeshShape) -> Self {
        Self {
            shape,
            color: Color::WHITE,
            metallic: 0.0,
            roughness: DEFAULT_ROUGHNESS,
            texture: None,
            filter: TextureFilter::Smooth,
            loaded: TextureSlot::EMPTY,
            rejected: None,
        }
    }

    /// The same mesh with its surface of `color`.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The same mesh, `metallic` from 0 to 1.
    #[must_use]
    pub const fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    /// The same mesh, `roughness` from 0 to 1.
    #[must_use]
    pub const fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }

    /// The same mesh showing the image at `path`, sampled with `filter`.
    #[must_use]
    pub fn with_texture(mut self, path: AssetPath, filter: TextureFilter) -> Self {
        self.texture = Some(path);
        self.filter = filter;
        self
    }

    /// The mesh of size 1 the shape is drawn from, and how much it is
    /// scaled along each axis. Tools that draw shapes themselves, like the
    /// editor, use it to build each kind of mesh once.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if a size is not positive and finite.
    pub fn unit_shape(&self) -> EngineResult<(MeshShape, Vec3)> {
        let (unit, scale) = match self.shape {
            MeshShape::Cube { size } => (MeshShape::Cube { size: Vec3::ONE }, size),
            MeshShape::Plane { size } => (
                MeshShape::Plane { size: Vec2::ONE },
                Vec3::new(size.x, 1.0, size.y),
            ),
            MeshShape::Sphere {
                radius,
                segments,
                rings,
            } => (
                MeshShape::Sphere {
                    radius: UNIT_SPHERE_RADIUS,
                    segments,
                    rings,
                },
                Vec3::ONE * (radius / UNIT_SPHERE_RADIUS),
            ),
        };
        if [scale.x, scale.y, scale.z]
            .iter()
            .all(|size| size.is_finite() && *size > 0.0)
        {
            Ok((unit, scale))
        } else {
            Err(EngineError::Rendering(format!(
                "the sizes of a mesh must be positive: {:?}",
                self.shape
            )))
        }
    }

    /// The material to draw with, its image loaded by `load` the first
    /// time; an image that cannot be loaded is passed to `on_error` once,
    /// and the mesh is drawn without it.
    pub(crate) fn material(
        &mut self,
        load: impl FnOnce(&AssetPath, TextureFilter) -> EngineResult<TextureId>,
        on_error: impl FnOnce(&EngineError),
    ) -> Material {
        let texture = self
            .texture
            .as_ref()
            .and_then(|path| self.loaded.texture(path, self.filter, load, on_error));
        Material {
            base_color: self.color,
            base_color_texture: texture,
            metallic: self.metallic,
            roughness: self.roughness,
        }
    }

    /// Passes `error` to `on_error` unless it was already reported for the
    /// current shape.
    pub(crate) fn reject(&mut self, error: &EngineError, on_error: impl FnOnce(&EngineError)) {
        if self.rejected != Some(self.shape) {
            self.rejected = Some(self.shape);
            on_error(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_drawn_from_unit_meshes_scaled() {
        let cube = MeshFile::new(MeshShape::Cube {
            size: Vec3::new(2.0, 1.0, 3.0),
        });
        let plane = MeshFile::new(MeshShape::Plane {
            size: Vec2::new(4.0, 5.0),
        });
        let sphere = MeshFile::new(MeshShape::Sphere {
            radius: 2.0,
            segments: 8,
            rings: 4,
        });

        assert_eq!(
            cube.unit_shape().unwrap(),
            (
                MeshShape::Cube { size: Vec3::ONE },
                Vec3::new(2.0, 1.0, 3.0)
            )
        );
        assert_eq!(plane.unit_shape().unwrap().1, Vec3::new(4.0, 1.0, 5.0));
        let (unit, scale) = sphere.unit_shape().unwrap();
        assert_eq!(
            unit,
            MeshShape::Sphere {
                radius: UNIT_SPHERE_RADIUS,
                segments: 8,
                rings: 4
            }
        );
        assert_eq!(scale, Vec3::ONE * 4.0);
    }

    #[test]
    fn sizes_must_be_positive() {
        for shape in [
            MeshShape::Cube {
                size: Vec3::new(1.0, 0.0, 1.0),
            },
            MeshShape::Plane {
                size: Vec2::new(-1.0, 1.0),
            },
            MeshShape::Sphere {
                radius: f32::NAN,
                segments: 8,
                rings: 4,
            },
        ] {
            assert!(MeshFile::new(shape).unit_shape().is_err(), "{shape:?}");
        }
    }

    #[test]
    fn the_material_has_the_image_once_it_loads() {
        let mut mesh = MeshFile::new(MeshShape::Cube { size: Vec3::ONE })
            .with_color(Color::rgb(1.0, 0.0, 0.0))
            .with_metallic(1.0)
            .with_roughness(0.2);
        assert_eq!(
            mesh.material(|_, _| unreachable!(), |_| {})
                .base_color_texture,
            None
        );

        mesh = mesh.with_texture(
            AssetPath::new("crate.png").unwrap(),
            TextureFilter::Pixelated,
        );
        let material = mesh.material(|_, _| Ok(TextureId::from_index(3)), |_| {});

        assert_eq!(material.base_color_texture, Some(TextureId::from_index(3)));
        assert_eq!(
            (material.base_color, material.metallic, material.roughness),
            (Color::rgb(1.0, 0.0, 0.0), 1.0, 0.2)
        );
    }

    #[test]
    fn rejections_are_reported_once_per_shape() {
        let mut mesh = MeshFile::new(MeshShape::Cube { size: Vec3::ZERO });
        let error = EngineError::Rendering("broken".to_owned());
        let mut reports = 0;

        mesh.reject(&error, |_| reports += 1);
        mesh.reject(&error, |_| reports += 1);
        mesh.shape = MeshShape::Cube {
            size: Vec3::new(0.0, 1.0, 1.0),
        };
        mesh.reject(&error, |_| reports += 1);

        assert_eq!(reports, 2);
    }
}
