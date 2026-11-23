//! The shapes 3D bodies collide with.

use star_core::{EngineError, EngineResult};
use star_math::Vec3;

use crate::shape::check_size;

/// The volume of a 3D body, centered on it, in world units.
///
/// Built through its constructors, which reject sizes that are not
/// positive and finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape3d {
    kind: ShapeKind3d,
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
        })
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
    fn a_capsule_holds_its_half_spheres() {
        assert!(matches!(
            Shape3d::capsule(0.5, 0.5),
            Err(EngineError::Physics(_))
        ));
    }
}
