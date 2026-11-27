//! The shapes 3D bodies collide with.

use star_core::{EngineError, EngineResult};
use star_math::Vec3;

use crate::shape::{check_size, scale_factor};

/// The volume of a 3D body, centered on it or at its
/// [`offset`](Self::offset), in world units.
///
/// Built through its constructors, which reject sizes that are not
/// positive and finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape3d {
    kind: ShapeKind3d,
    /// Where its center is from the body's, in the body's space.
    offset: Vec3,
}

/// What a [`Shape3d`] is, with its validated sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeKind3d {
    /// A sphere.
    Sphere {
        /// Distance from the center to the surface.
        radius: f32,
    },
    /// A box, aligned with the body (it turns with it).
    Box {
        /// Width (x), height (y) and depth (z).
        size: Vec3,
    },
    /// A vertical capsule: a cylinder with half spheres on top and bottom,
    /// good for characters (they slide over steps and corners).
    Capsule {
        /// Total height, half spheres included.
        height: f32,
        /// Radius of the cylinder and the half spheres.
        radius: f32,
    },
    /// The triangles of the mesh or model its node shows, which the
    /// physics is given apart ([`TriangleMesh`](crate::TriangleMesh)):
    /// exact for static and kinematic bodies, wrapped in their convex hull
    /// for dynamic ones.
    Mesh,
}

impl Shape3d {
    /// A sphere of `radius`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if `radius` is not positive and finite.
    pub fn sphere(radius: f32) -> EngineResult<Self> {
        check_size("sphere radius", radius)?;
        Ok(Self {
            kind: ShapeKind3d::Sphere { radius },
            offset: Vec3::ZERO,
        })
    }

    /// A box of `size` (width, height and depth).
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a side is not positive and finite.
    pub fn cuboid(size: Vec3) -> EngineResult<Self> {
        check_size("box width", size.x)?;
        check_size("box height", size.y)?;
        check_size("box depth", size.z)?;
        Ok(Self {
            kind: ShapeKind3d::Box { size },
            offset: Vec3::ZERO,
        })
    }

    /// A vertical capsule of `height` (half spheres included) and `radius`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a size is not positive and finite, or the
    /// height is smaller than the two half spheres (`2 * radius`).
    pub fn capsule(height: f32, radius: f32) -> EngineResult<Self> {
        check_size("capsule height", height)?;
        check_size("capsule radius", radius)?;
        if height < 2.0 * radius {
            return Err(EngineError::Physics(format!(
                "a capsule of radius {radius} must be at least {} high, not {height}",
                2.0 * radius
            )));
        }
        Ok(Self {
            kind: ShapeKind3d::Capsule { height, radius },
            offset: Vec3::ZERO,
        })
    }

    /// The shape stretched by `scale`, the scale of its node in the world
    /// (mirroring is ignored): a box along each axis, a sphere by the
    /// largest one, and a capsule with its height by `y` and its radius by
    /// the larger of `x` and `z`, kept at least as high as its half
    /// spheres. A mesh's triangles are stretched when the physics is given
    /// them. The offset is stretched too, and mirrored by a negative
    /// factor. A factor smaller than
    /// [`MIN_SHAPE_SCALE`](crate::MIN_SHAPE_SCALE) counts as that one.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a factor is not finite.
    pub fn scaled(self, scale: Vec3) -> EngineResult<Self> {
        let offset = Vec3::new(
            self.offset.x * scale.x,
            self.offset.y * scale.y,
            self.offset.z * scale.z,
        );
        Ok(self.scaled_size(scale)?.with_offset_unchecked(offset))
    }

    fn scaled_size(self, scale: Vec3) -> EngineResult<Self> {
        let [x, y, z] = [scale.x, scale.y, scale.z].map(scale_factor);
        let (x, y, z) = (x?, y?, z?);
        match self.kind {
            ShapeKind3d::Sphere { radius } => Self::sphere(radius * x.max(y).max(z)),
            ShapeKind3d::Box { size } => {
                Self::cuboid(Vec3::new(size.x * x, size.y * y, size.z * z))
            }
            ShapeKind3d::Capsule { height, radius } => {
                let radius = radius * x.max(z);
                Self::capsule((height * y).max(2.0 * radius), radius)
            }
            // Its triangles are stretched when they are given.
            ShapeKind3d::Mesh => Ok(self),
        }
    }

    /// The same shape with its center `offset` from the body's, in the
    /// body's space (before its node's scale, which stretches it too).
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if `offset` is not finite.
    pub fn with_offset(self, offset: Vec3) -> EngineResult<Self> {
        if [offset.x, offset.y, offset.z]
            .iter()
            .all(|part| part.is_finite())
        {
            Ok(self.with_offset_unchecked(offset))
        } else {
            Err(EngineError::Physics(format!(
                "the offset of a body must be finite, not {offset:?}"
            )))
        }
    }

    const fn with_offset_unchecked(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    /// Where its center is from the body's, in the body's space.
    #[must_use]
    pub const fn offset(&self) -> Vec3 {
        self.offset
    }

    /// The shape of the triangles of the mesh or model its node shows.
    #[must_use]
    pub const fn mesh() -> Self {
        Self {
            kind: ShapeKind3d::Mesh,
            offset: Vec3::ZERO,
        }
    }

    /// What the shape is, with its sizes.
    #[must_use]
    pub const fn kind(&self) -> ShapeKind3d {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sizes_make_shapes() {
        assert_eq!(
            Shape3d::sphere(0.5).unwrap().kind(),
            ShapeKind3d::Sphere { radius: 0.5 }
        );
        assert_eq!(
            Shape3d::cuboid(Vec3::new(2.0, 1.0, 3.0)).unwrap().kind(),
            ShapeKind3d::Box {
                size: Vec3::new(2.0, 1.0, 3.0)
            }
        );
        assert!(Shape3d::capsule(1.0, 0.5).is_ok());
    }

    #[test]
    fn sizes_must_be_positive_and_finite() {
        assert!(Shape3d::sphere(0.0).is_err());
        assert!(Shape3d::sphere(f32::NAN).is_err());
        assert!(Shape3d::cuboid(Vec3::new(1.0, 1.0, -1.0)).is_err());
        assert!(Shape3d::cuboid(Vec3::new(1.0, f32::INFINITY, 1.0)).is_err());
        assert!(Shape3d::capsule(1.0, 0.0).is_err());
    }

    #[test]
    fn shapes_stretch_with_their_nodes() {
        let scale = Vec3::new(3.0, 1.0, -2.0);

        assert_eq!(
            Shape3d::cuboid(Vec3::ONE).unwrap().scaled(scale).unwrap(),
            Shape3d::cuboid(Vec3::new(3.0, 1.0, 2.0)).unwrap()
        );
        assert_eq!(
            Shape3d::sphere(0.5).unwrap().scaled(scale).unwrap(),
            Shape3d::sphere(1.5).unwrap()
        );
        assert_eq!(
            Shape3d::capsule(2.0, 0.5)
                .unwrap()
                .scaled(Vec3::new(1.0, 2.0, 1.5))
                .unwrap(),
            Shape3d::capsule(4.0, 0.75).unwrap()
        );
        assert_eq!(
            Shape3d::capsule(2.0, 0.5).unwrap().scaled(scale).unwrap(),
            Shape3d::capsule(3.0, 1.5).unwrap(),
            "the capsule keeps room for its half spheres"
        );
        assert!(Shape3d::sphere(1.0).unwrap().scaled(Vec3::ZERO).is_ok());
        assert!(
            Shape3d::sphere(1.0)
                .unwrap()
                .scaled(Vec3::new(1.0, f32::INFINITY, 1.0))
                .is_err()
        );
    }

    #[test]
    fn offsets_stretch_and_mirror_with_the_node() {
        let shape = Shape3d::sphere(0.5)
            .unwrap()
            .with_offset(Vec3::new(0.0, 1.0, 2.0))
            .unwrap();

        let scaled = shape.scaled(Vec3::new(1.0, 2.0, -1.0)).unwrap();

        assert_eq!(scaled.offset(), Vec3::new(0.0, 2.0, -2.0));
        assert_eq!(Shape3d::mesh().offset(), Vec3::ZERO);
        assert!(shape.with_offset(Vec3::new(f32::NAN, 0.0, 0.0)).is_err());
    }

    #[test]
    fn a_capsule_holds_its_half_spheres() {
        assert!(matches!(
            Shape3d::capsule(0.5, 0.5),
            Err(EngineError::Physics(_))
        ));
    }
}
