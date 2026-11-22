//! Two-dimensional affine transform.

use std::ops::Mul;

use crate::Vec2;

/// A 2D transform that moves, rotates, scales (and, when combined, may
/// skew) points: where the x and y axes go, plus a translation.
///
/// Combining with `*` applies the right side first: `parent * child` is the
/// child's transform seen from the parent's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2 {
    x_axis: Vec2,
    y_axis: Vec2,
    translation: Vec2,
}

impl Affine2 {
    /// The transform that leaves every point where it is.
    pub const IDENTITY: Self = Self {
        x_axis: Vec2::new(1.0, 0.0),
        y_axis: Vec2::new(0.0, 1.0),
        translation: Vec2::ZERO,
    };

    /// Scales by `scale`, then rotates `rotation` radians counterclockwise,
    /// then moves by `translation`.
    #[must_use]
    pub fn from_scale_rotation_translation(scale: Vec2, rotation: f32, translation: Vec2) -> Self {
        let (sin, cos) = rotation.sin_cos();
        Self {
            x_axis: Vec2::new(cos, sin) * scale.x,
            y_axis: Vec2::new(-sin, cos) * scale.y,
            translation,
        }
    }

    /// Where `point` goes.
    #[must_use]
    pub fn transform_point(self, point: Vec2) -> Vec2 {
        self.transform_vector(point) + self.translation
    }

    /// Where the direction or offset `vector` goes: like a point, but not moved.
    #[must_use]
    pub fn transform_vector(self, vector: Vec2) -> Vec2 {
        self.x_axis * vector.x + self.y_axis * vector.y
    }

    /// Where the origin goes.
    #[must_use]
    pub const fn translation(self) -> Vec2 {
        self.translation
    }

    /// The counterclockwise angle of the x axis, in radians from -π to π.
    #[must_use]
    pub fn rotation(self) -> f32 {
        self.x_axis.y.atan2(self.x_axis.x)
    }

    /// How much the axes are stretched; `y` is negative when the transform
    /// mirrors. Exact for transforms without skew.
    #[must_use]
    pub fn scale(self) -> Vec2 {
        let determinant = self.x_axis.x * self.y_axis.y - self.x_axis.y * self.y_axis.x;
        let y_length = self.y_axis.length();
        Vec2::new(
            self.x_axis.length(),
            if determinant < 0.0 {
                -y_length
            } else {
                y_length
            },
        )
    }
}

impl Default for Affine2 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Affine2 {
    type Output = Self;

    fn mul(self, inner: Self) -> Self {
        Self {
            x_axis: self.transform_vector(inner.x_axis),
            y_axis: self.transform_vector(inner.y_axis),
            translation: self.transform_point(inner.translation),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_close(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < EPSILON,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn identity_leaves_points_unchanged() {
        let point = Vec2::new(3.0, -2.0);

        assert_close(Affine2::IDENTITY.transform_point(point), point);
        assert_eq!(Affine2::default(), Affine2::IDENTITY);
    }

    #[test]
    fn points_are_scaled_then_rotated_then_moved() {
        let transform = Affine2::from_scale_rotation_translation(
            Vec2::new(2.0, 1.0),
            FRAC_PI_2,
            Vec2::new(10.0, 0.0),
        );

        assert_close(
            transform.transform_point(Vec2::new(1.0, 0.0)),
            Vec2::new(10.0, 2.0),
        );
        assert_close(
            transform.transform_vector(Vec2::new(1.0, 0.0)),
            Vec2::new(0.0, 2.0),
        );
    }

    #[test]
    fn combining_applies_the_inner_transform_first() {
        let parent =
            Affine2::from_scale_rotation_translation(Vec2::ONE, FRAC_PI_2, Vec2::new(5.0, 0.0));
        let child = Affine2::from_scale_rotation_translation(Vec2::ONE, 0.0, Vec2::new(1.0, 0.0));

        let combined = parent * child;

        assert_close(combined.translation(), Vec2::new(5.0, 1.0));
        assert_close(
            combined.transform_point(Vec2::new(1.0, 0.0)),
            parent.transform_point(child.transform_point(Vec2::new(1.0, 0.0))),
        );
    }

    #[test]
    fn rotation_and_scale_are_recovered() {
        let transform = Affine2::from_scale_rotation_translation(
            Vec2::new(2.0, -3.0),
            0.5,
            Vec2::new(1.0, 1.0),
        );

        assert!((transform.rotation() - 0.5).abs() < EPSILON);
        assert_close(transform.scale(), Vec2::new(2.0, -3.0));
    }
}
