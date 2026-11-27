//! The shapes bodies collide with.

use star_core::{EngineError, EngineResult};
use star_math::Vec2;

/// The outline of a body, centered on it, in world units.
///
/// Built through its constructors, which reject sizes that are not
/// positive and finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape2d {
    kind: ShapeKind,
}

/// What a [`Shape2d`] is, with its validated sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeKind {
    /// A circle.
    Circle {
        /// Distance from the center to the edge.
        radius: f32,
    },
    /// A rectangle, aligned with the body (it turns with it).
    Rectangle {
        /// Width and height.
        size: Vec2,
    },
    /// A vertical capsule: a rectangle with half circles on top and bottom,
    /// good for characters (they slide over steps and corners).
    Capsule {
        /// Total height, half circles included.
        height: f32,
        /// Radius of the half circles, which is half the width.
        radius: f32,
    },
}

impl Shape2d {
    /// A circle of `radius`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if `radius` is not positive and finite.
    pub fn circle(radius: f32) -> EngineResult<Self> {
        check_size("circle radius", radius)?;
        Ok(Self {
            kind: ShapeKind::Circle { radius },
        })
    }

    /// A rectangle of `size` (width and height).
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a side is not positive and finite.
    pub fn rectangle(size: Vec2) -> EngineResult<Self> {
        check_size("rectangle width", size.x)?;
        check_size("rectangle height", size.y)?;
        Ok(Self {
            kind: ShapeKind::Rectangle { size },
        })
    }

    /// A vertical capsule of `height` (half circles included) and `radius`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a size is not positive and finite, or the
    /// height is smaller than the two half circles (`2 * radius`).
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
            kind: ShapeKind::Capsule { height, radius },
        })
    }

    /// The shape stretched by `scale`, the scale of its node in the world
    /// (mirroring is ignored): a rectangle along each axis, a circle by
    /// the larger one, and a capsule with its height by `y` and its radius
    /// by `x`, kept at least as high as its half circles. A factor smaller
    /// than [`MIN_SHAPE_SCALE`] counts as that one, so a node shrunk to
    /// nothing keeps a tiny body.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a factor is not finite.
    pub fn scaled(self, scale: Vec2) -> EngineResult<Self> {
        let [x, y] = [scale.x, scale.y].map(scale_factor);
        let (x, y) = (x?, y?);
        match self.kind {
            ShapeKind::Circle { radius } => Self::circle(radius * x.max(y)),
            ShapeKind::Rectangle { size } => Self::rectangle(Vec2::new(size.x * x, size.y * y)),
            ShapeKind::Capsule { height, radius } => {
                let radius = radius * x;
                Self::capsule((height * y).max(2.0 * radius), radius)
            }
        }
    }

    /// What the shape is, with its sizes.
    #[must_use]
    pub const fn kind(&self) -> ShapeKind {
        self.kind
    }
}

/// The smallest factor shapes are scaled by: a node scaled to nothing
/// still has a body, too small to be met.
pub const MIN_SHAPE_SCALE: f32 = 1e-3;

/// A factor of a node's scale as shapes are stretched by it: without its
/// sign, and at least [`MIN_SHAPE_SCALE`].
pub(crate) fn scale_factor(factor: f32) -> EngineResult<f32> {
    if factor.is_finite() {
        Ok(factor.abs().max(MIN_SHAPE_SCALE))
    } else {
        Err(EngineError::Physics(format!(
            "a body cannot be scaled by {factor}"
        )))
    }
}

/// Checks that a size of a shape is positive and finite.
pub(crate) fn check_size(what: &str, size: f32) -> EngineResult<()> {
    if size.is_finite() && size > 0.0 {
        Ok(())
    } else {
        Err(EngineError::Physics(format!(
            "the {what} must be positive, not {size}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sizes_make_shapes() {
        assert_eq!(
            Shape2d::circle(0.5).unwrap().kind(),
            ShapeKind::Circle { radius: 0.5 }
        );
        assert_eq!(
            Shape2d::rectangle(Vec2::new(2.0, 1.0)).unwrap().kind(),
            ShapeKind::Rectangle {
                size: Vec2::new(2.0, 1.0)
            }
        );
        assert!(Shape2d::capsule(1.0, 0.5).is_ok());
    }

    #[test]
    fn sizes_must_be_positive_and_finite() {
        assert!(Shape2d::circle(0.0).is_err());
        assert!(Shape2d::circle(-1.0).is_err());
        assert!(Shape2d::circle(f32::NAN).is_err());
        assert!(Shape2d::rectangle(Vec2::new(1.0, f32::INFINITY)).is_err());
        assert!(Shape2d::capsule(1.0, 0.0).is_err());
    }

    #[test]
    fn shapes_stretch_with_their_nodes() {
        let scale = Vec2::new(3.0, -2.0);

        assert_eq!(
            Shape2d::rectangle(Vec2::ONE)
                .unwrap()
                .scaled(scale)
                .unwrap(),
            Shape2d::rectangle(Vec2::new(3.0, 2.0)).unwrap()
        );
        assert_eq!(
            Shape2d::circle(0.5).unwrap().scaled(scale).unwrap(),
            Shape2d::circle(1.5).unwrap()
        );
        assert_eq!(
            Shape2d::capsule(2.0, 0.5).unwrap().scaled(scale).unwrap(),
            Shape2d::capsule(4.0, 1.5).unwrap()
        );
    }

    #[test]
    fn scaled_shapes_stay_valid() {
        let capsule = Shape2d::capsule(1.0, 0.5).unwrap();

        let flattened = capsule.scaled(Vec2::new(1.0, 0.1)).unwrap();
        let vanished = capsule.scaled(Vec2::ZERO).unwrap();

        assert_eq!(flattened, Shape2d::capsule(1.0, 0.5).unwrap());
        assert!(matches!(vanished.kind(), ShapeKind::Capsule { radius, .. } if radius > 0.0));
        assert!(capsule.scaled(Vec2::new(f32::NAN, 1.0)).is_err());
    }

    #[test]
    fn a_capsule_holds_its_half_circles() {
        let error = Shape2d::capsule(0.5, 0.5).unwrap_err();

        assert!(matches!(error, EngineError::Physics(_)));
    }
}
