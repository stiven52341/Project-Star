//! Three-dimensional vector.

use std::ops::{Add, Mul, Neg, Sub};

/// A 3D vector, used for positions, directions and scales in 3D games.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    /// X component.
    pub x: f32,
    /// Y component (up axis).
    pub y: f32,
    /// Z component.
    pub z: f32,
}

impl Vec3 {
    /// Vector with all components set to zero.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// Vector with all components set to one.
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    /// World up direction.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Creates a vector from its components.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Dot product of two vectors.
    #[must_use]
    pub fn dot(self, other: Self) -> f32 {
        self.x
            .mul_add(other.x, self.y.mul_add(other.y, self.z * other.z))
    }

    /// Cross product, perpendicular to both vectors.
    #[must_use]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
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

impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_of_x_and_y_is_z() {
        let x_axis = Vec3::new(1.0, 0.0, 0.0);
        let y_axis = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(x_axis.cross(y_axis), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn normalized_vector_has_unit_length() {
        let unit = Vec3::new(2.0, 3.0, 6.0)
            .normalized()
            .expect("non-zero vector");

        assert!((unit.length() - 1.0).abs() < 1e-6);
    }
}
