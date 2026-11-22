//! Two-dimensional vector.

use std::ops::{Add, Mul, Neg, Sub};

/// A 2D vector, used for positions, directions and sizes in 2D games.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

impl Vec2 {
    /// Vector with both components set to zero.
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// Vector with both components set to one.
    pub const ONE: Self = Self::new(1.0, 1.0);

    /// Creates a vector from its components.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Dot product of two vectors.
    #[must_use]
    pub fn dot(self, other: Self) -> f32 {
        self.x.mul_add(other.x, self.y * other.y)
    }

    /// Euclidean length of the vector.
    #[must_use]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Unit vector with the same direction, or `None` if the length is zero.
    #[must_use]
    pub fn normalized(self) -> Option<Self> {
        let length = self.length();
        (length > 0.0).then(|| self * length.recip())
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_three_four_is_five() {
        assert!((Vec2::new(3.0, 4.0).length() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn arithmetic_operators_work_component_wise() {
        let sum = Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0);
        let difference = Vec2::new(5.0, 5.0) - Vec2::ONE;

        assert_eq!(sum, Vec2::new(4.0, 6.0));
        assert_eq!(difference, Vec2::new(4.0, 4.0));
        assert_eq!(-Vec2::ONE * 2.0, Vec2::new(-2.0, -2.0));
    }

    #[test]
    fn normalizing_zero_vector_returns_none() {
        assert_eq!(Vec2::ZERO.normalized(), None);
    }
}
