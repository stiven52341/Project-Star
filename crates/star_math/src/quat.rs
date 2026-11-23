//! Rotations in 3D, as unit quaternions.

use std::ops::Mul;

use crate::{Mat4, Vec3};

/// A rotation in 3D. Combining and interpolating quaternions never gets
/// stuck the way three angles do (gimbal lock), so 3D nodes keep their
/// rotation as one; angles are only for people ([`Quat::from_euler`]).
///
/// Built by its constructors it always has length 1; `a * b` rotates by
/// `b` first, then by `a`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

/// Beyond this the sine of the pitch is treated as straight up or down,
/// where yaw and roll turn around the same axis.
const GIMBAL_LIMIT: f32 = 0.999_999;

impl Quat {
    /// No rotation.
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// A rotation of `angle` radians around `axis`, counterclockwise when
    /// the axis points at the viewer; `None` if the axis has no length.
    #[must_use]
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Option<Self> {
        let axis = axis.normalized()?;
        let (sin, cos) = (angle / 2.0).sin_cos();
        Some(Self {
            x: axis.x * sin,
            y: axis.y * sin,
            z: axis.z * sin,
            w: cos,
        })
    }

    /// A rotation of `angle` radians around the x axis (pitch: up and down).
    #[must_use]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (sin, cos) = (angle / 2.0).sin_cos();
        Self {
            x: sin,
            y: 0.0,
            z: 0.0,
            w: cos,
        }
    }

    /// A rotation of `angle` radians around the y axis (yaw: left and right).
    #[must_use]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (sin, cos) = (angle / 2.0).sin_cos();
        Self {
            x: 0.0,
            y: sin,
            z: 0.0,
            w: cos,
        }
    }

    /// A rotation of `angle` radians around the z axis (roll).
    #[must_use]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (sin, cos) = (angle / 2.0).sin_cos();
        Self {
            x: 0.0,
            y: 0.0,
            z: sin,
            w: cos,
        }
    }

    /// The rotation of three angles in radians, `angles.x` around x
    /// (pitch), `angles.y` around y (yaw) and `angles.z` around z (roll),
    /// applied roll first, then pitch, then yaw: a camera or character
    /// turns left and right, then looks up and down.
    #[must_use]
    pub fn from_euler(angles: Vec3) -> Self {
        Self::from_rotation_y(angles.y)
            * Self::from_rotation_x(angles.x)
            * Self::from_rotation_z(angles.z)
    }

    /// The angles of [`Quat::from_euler`] that give this rotation: pitch
    /// from -π/2 to π/2, yaw and roll from -π to π. Looking straight up or
    /// down, the roll is 0 and the yaw carries the whole turn.
    #[must_use]
    pub fn to_euler(self) -> Vec3 {
        let Self { x, y, z, w } = self;
        // Elements of the rotation matrix, by row and column.
        let m02 = 2.0 * (x * z + w * y);
        let m10 = 2.0 * (x * y + w * z);
        let m11 = 1.0 - 2.0 * (x * x + z * z);
        let m12 = 2.0 * (y * z - w * x);
        let m20 = 2.0 * (x * z - w * y);
        let m22 = 1.0 - 2.0 * (x * x + y * y);
        let m00 = 1.0 - 2.0 * (y * y + z * z);
        let sin_pitch = (-m12).clamp(-1.0, 1.0);
        let pitch = sin_pitch.asin();
        if sin_pitch.abs() < GIMBAL_LIMIT {
            Vec3::new(pitch, m02.atan2(m22), m10.atan2(m11))
        } else {
            Vec3::new(pitch, (-m20).atan2(m00), 0.0)
        }
    }

    /// The rotation of `matrix`, leaving its scale and translation out;
    /// `None` if an axis is scaled to nothing. A mirrored matrix (with a
    /// negative scale) has no rotation that matches it.
    #[must_use]
    pub fn from_mat4(matrix: Mat4) -> Option<Self> {
        let [x_axis, y_axis, z_axis, _] = matrix.to_columns();
        let [x_axis, y_axis, z_axis] =
            [x_axis, y_axis, z_axis].map(|[x, y, z, _]| Vec3::new(x, y, z).normalized());
        let (x_axis, y_axis, z_axis) = (x_axis?, y_axis?, z_axis?);
        // Elements of the rotation matrix, by row and column.
        let (m00, m10, m20) = (x_axis.x, x_axis.y, x_axis.z);
        let (m01, m11, m21) = (y_axis.x, y_axis.y, y_axis.z);
        let (m02, m12, m22) = (z_axis.x, z_axis.y, z_axis.z);
        // The largest of the four components is computed first, so it is
        // never divided by a number close to zero.
        let trace = m00 + m11 + m22;
        let [x, y, z, w] = if trace > 0.0 {
            let four_w = (trace + 1.0).sqrt() * 2.0;
            [
                (m21 - m12) / four_w,
                (m02 - m20) / four_w,
                (m10 - m01) / four_w,
                four_w / 4.0,
            ]
        } else if m00 > m11 && m00 > m22 {
            let four_x = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            [
                four_x / 4.0,
                (m01 + m10) / four_x,
                (m02 + m20) / four_x,
                (m21 - m12) / four_x,
            ]
        } else if m11 > m22 {
            let four_y = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            [
                (m01 + m10) / four_y,
                four_y / 4.0,
                (m12 + m21) / four_y,
                (m02 - m20) / four_y,
            ]
        } else {
            let four_z = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            [
                (m02 + m20) / four_z,
                (m12 + m21) / four_z,
                four_z / 4.0,
                (m10 - m01) / four_z,
            ]
        };
        Self::from_xyzw([x, y, z, w])
    }

    /// The rotation of the four numbers `[x, y, z, w]` of a quaternion,
    /// scaled to length 1; `None` if they have no length or are not finite.
    #[must_use]
    pub fn from_xyzw([x, y, z, w]: [f32; 4]) -> Option<Self> {
        Self { x, y, z, w }.normalized()
    }

    /// The four numbers `[x, y, z, w]` of the quaternion.
    #[must_use]
    pub const fn to_xyzw(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    /// The same rotation with length 1 again, after many combinations
    /// rounded it; `None` if it has no length or is not finite.
    #[must_use]
    pub fn normalized(self) -> Option<Self> {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        (length > 0.0 && length.is_finite()).then(|| {
            let scale = length.recip();
            Self {
                x: self.x * scale,
                y: self.y * scale,
                z: self.z * scale,
                w: self.w * scale,
            }
        })
    }

    /// The opposite rotation.
    #[must_use]
    pub const fn inverse(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// `vector` turned by this rotation.
    #[must_use]
    pub fn rotate(self, vector: Vec3) -> Vec3 {
        let axis = Vec3::new(self.x, self.y, self.z);
        let twice_cross = axis.cross(vector) * 2.0;
        vector + twice_cross * self.w + axis.cross(twice_cross)
    }

    /// The rotation as a matrix.
    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        let [x_axis, y_axis, z_axis] =
            [Vec3::new(1.0, 0.0, 0.0), Vec3::UP, Vec3::new(0.0, 0.0, 1.0)]
                .map(|axis| self.rotate(axis));
        Mat4::from_columns([
            [x_axis.x, x_axis.y, x_axis.z, 0.0],
            [y_axis.x, y_axis.y, y_axis.z, 0.0],
            [z_axis.x, z_axis.y, z_axis.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Quat {
    type Output = Self;

    /// Rotates by `other` first, then by `self`.
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    use super::*;

    const EPSILON: f32 = 1e-5;
    const GIMBAL_TOLERANCE: f32 = 1e-2;

    fn assert_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual - expected).length() < EPSILON,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn rotations_turn_counterclockwise_around_their_axis() {
        let quarter_y = Quat::from_rotation_y(FRAC_PI_2);

        assert_close(
            quarter_y.rotate(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(0.0, 0.0, -1.0),
        );
        assert_close(
            Quat::from_rotation_x(FRAC_PI_2).rotate(Vec3::UP),
            Vec3::new(0.0, 0.0, 1.0),
        );
        assert_close(
            Quat::from_rotation_z(FRAC_PI_2).rotate(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::UP,
        );
        let around_y = Quat::from_axis_angle(Vec3::new(0.0, 3.0, 0.0), FRAC_PI_2).unwrap();
        assert_close(
            around_y.rotate(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(0.0, 0.0, -1.0),
        );
        assert_eq!(Quat::from_axis_angle(Vec3::ZERO, 1.0), None);
    }

    #[test]
    fn rotations_agree_with_matrices() {
        let point = Vec3::new(0.3, -2.0, 1.5);
        for (quat, matrix) in [
            (Quat::from_rotation_x(0.7), Mat4::from_rotation_x(0.7)),
            (Quat::from_rotation_y(-1.2), Mat4::from_rotation_y(-1.2)),
            (Quat::from_rotation_z(2.5), Mat4::from_rotation_z(2.5)),
        ] {
            assert_close(quat.rotate(point), matrix.transform_point(point));
            assert_close(
                quat.to_mat4().transform_point(point),
                matrix.transform_point(point),
            );
        }
    }

    #[test]
    fn products_rotate_right_to_left() {
        let point = Vec3::new(1.0, 0.0, 0.0);
        let (first, then) = (
            Quat::from_rotation_z(FRAC_PI_2),
            Quat::from_rotation_y(FRAC_PI_2),
        );

        let combined = then * first;

        assert_close(combined.rotate(point), then.rotate(first.rotate(point)));
        assert_close((combined * combined.inverse()).rotate(point), point);
    }

    #[test]
    fn euler_angles_read_back_the_same() {
        for angles in [
            Vec3::new(0.3, -1.1, 0.4),
            Vec3::new(-FRAC_PI_4, 2.8, -3.0),
            Vec3::new(0.0, PI / 3.0, 0.0),
        ] {
            let rotation = Quat::from_euler(angles);

            assert_close(rotation.to_euler(), angles);
        }
    }

    #[test]
    fn looking_straight_up_keeps_the_rotation() {
        let rotation = Quat::from_euler(Vec3::new(FRAC_PI_2, 0.5, 0.3));
        let point = Vec3::new(1.0, 2.0, 3.0);

        let again = Quat::from_euler(rotation.to_euler());

        // At the pole f32 tells pitches apart only to about a thousandth.
        let error = (again.rotate(point) - rotation.rotate(point)).length();
        assert!(error < GIMBAL_TOLERANCE, "{error}");
    }

    #[test]
    fn quaternions_round_trip_through_their_numbers() {
        let rotation = Quat::from_euler(Vec3::new(0.3, -1.1, 0.4));

        let again = Quat::from_xyzw(rotation.to_xyzw()).unwrap();
        let point = Vec3::new(1.0, 2.0, 3.0);
        assert_close(again.rotate(point), rotation.rotate(point));
        assert_eq!(Quat::from_xyzw([0.0, 0.0, 0.0, 2.0]), Some(Quat::IDENTITY));
        assert_eq!(Quat::from_xyzw([f32::NAN, 0.0, 0.0, 1.0]), None);
    }

    #[test]
    fn matrices_give_back_their_rotation() {
        let point = Vec3::new(1.0, 2.0, 3.0);
        for angles in [
            Vec3::ZERO,
            Vec3::new(0.3, -1.1, 0.4),
            Vec3::new(0.0, PI, 0.0),
            Vec3::new(PI, 0.0, 0.0),
            Vec3::new(0.0, 0.0, PI),
            Vec3::new(-FRAC_PI_4, 2.8, -3.0),
        ] {
            let rotation = Quat::from_euler(angles);
            let matrix = Mat4::from_scale_rotation_translation(
                Vec3::new(2.0, 0.5, 3.0),
                rotation,
                Vec3::new(4.0, 5.0, 6.0),
            );

            let read = Quat::from_mat4(matrix).unwrap();

            assert_close(read.rotate(point), rotation.rotate(point));
        }
        assert_eq!(
            Quat::from_mat4(Mat4::from_scale(Vec3::new(0.0, 1.0, 1.0))),
            None
        );
    }

    #[test]
    fn normalizing_restores_unit_length() {
        let stretched = Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 2.0,
        };

        assert_eq!(stretched.normalized(), Some(Quat::IDENTITY));
        assert_eq!(
            Quat {
                w: 0.0,
                ..Quat::IDENTITY
            }
            .normalized(),
            None
        );
    }
}
