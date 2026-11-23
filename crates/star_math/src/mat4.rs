//! 4x4 matrix for 3D transforms and projections.

use std::ops::Mul;

use crate::{Quat, Vec3};

/// Column-major 4x4 matrix.
///
/// Uses a right-handed coordinate system (+Y up, camera looking towards -Z)
/// and maps depth to the `0..=1` range expected by modern graphics APIs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    columns: [[f32; 4]; 4],
}

impl Mat4 {
    /// Matrix that leaves every point unchanged.
    pub const IDENTITY: Self = Self::from_columns([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// Creates a matrix from its four columns.
    #[must_use]
    pub const fn from_columns(columns: [[f32; 4]; 4]) -> Self {
        Self { columns }
    }

    /// The four columns of the matrix, in the layout GPU shaders expect.
    #[must_use]
    pub const fn to_columns(self) -> [[f32; 4]; 4] {
        self.columns
    }

    /// Moves points by `offset`.
    #[must_use]
    pub const fn from_translation(offset: Vec3) -> Self {
        Self::from_columns([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [offset.x, offset.y, offset.z, 1.0],
        ])
    }

    /// Rotates points around the X axis by `angle` radians.
    #[must_use]
    pub fn from_rotation_x(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::from_columns([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, cos, sin, 0.0],
            [0.0, -sin, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Rotates points around the Y axis by `angle` radians.
    #[must_use]
    pub fn from_rotation_y(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::from_columns([
            [cos, 0.0, -sin, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [sin, 0.0, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Rotates points around the Z axis by `angle` radians.
    #[must_use]
    pub fn from_rotation_z(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::from_columns([
            [cos, sin, 0.0, 0.0],
            [-sin, cos, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Scales points by `factors` along each axis.
    #[must_use]
    pub const fn from_scale(factors: Vec3) -> Self {
        Self::from_columns([
            [factors.x, 0.0, 0.0, 0.0],
            [0.0, factors.y, 0.0, 0.0],
            [0.0, 0.0, factors.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// View matrix of a camera at `eye` looking at `target`.
    ///
    /// Returns `None` if `eye` and `target` coincide or the view direction is
    /// parallel to `up`, because no orientation can be derived.
    #[must_use]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Option<Self> {
        let forward = (target - eye).normalized()?;
        let right = forward.cross(up).normalized()?;
        let camera_up = right.cross(forward);
        Some(Self::from_columns([
            [right.x, camera_up.x, -forward.x, 0.0],
            [right.y, camera_up.y, -forward.y, 0.0],
            [right.z, camera_up.z, -forward.z, 0.0],
            [-right.dot(eye), -camera_up.dot(eye), forward.dot(eye), 1.0],
        ]))
    }

    /// Perspective projection: distant objects look smaller.
    ///
    /// `vertical_fov` is in radians and `aspect_ratio` is width / height.
    #[must_use]
    pub fn perspective(vertical_fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let focal_length = 1.0 / (vertical_fov / 2.0).tan();
        let depth_scale = far / (near - far);
        Self::from_columns([
            [focal_length / aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, focal_length, 0.0, 0.0],
            [0.0, 0.0, depth_scale, -1.0],
            [0.0, 0.0, near * depth_scale, 0.0],
        ])
    }

    /// Orthographic projection: objects keep their size regardless of distance.
    ///
    /// Maps the box delimited by the given planes to the visible screen area.
    #[must_use]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let width = right - left;
        let height = top - bottom;
        let depth = near - far;
        Self::from_columns([
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0 / depth, 0.0],
            [
                -(right + left) / width,
                -(top + bottom) / height,
                near / depth,
                1.0,
            ],
        ])
    }

    /// Applies the matrix to a point (with perspective division).
    #[must_use]
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        let [x, y, z, w] = self.transform([point.x, point.y, point.z, 1.0]);
        Vec3::new(x / w, y / w, z / w)
    }

    /// Scales by `scale`, then rotates by `rotation`, then moves by
    /// `translation`: where a 3D node is relative to its parent.
    #[must_use]
    pub fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        let [x_axis, y_axis, z_axis, _] = rotation.to_mat4().columns;
        let scaled = |axis: [f32; 4], factor: f32| axis.map(|value| value * factor);
        Self::from_columns([
            scaled(x_axis, scale.x),
            scaled(y_axis, scale.y),
            scaled(z_axis, scale.z),
            [translation.x, translation.y, translation.z, 1.0],
        ])
    }

    /// Where the matrix moves the origin (without perspective).
    #[must_use]
    pub const fn translation(self) -> Vec3 {
        let [x, y, z, _] = self.columns[3];
        Vec3::new(x, y, z)
    }

    /// The matrix with its rows as columns.
    #[must_use]
    pub fn transposed(self) -> Self {
        let columns = self.columns;
        Self::from_columns(std::array::from_fn(|column| {
            std::array::from_fn(|row| columns[row][column])
        }))
    }

    /// The matrix that undoes this one, or `None` if it cannot be undone
    /// (it flattens space, e.g. a scale of zero) or is not made of finite
    /// numbers.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        // Cofactors of the 2x2 minors, as in Laplace's expansion.
        let [
            [a00, a01, a02, a03],
            [a10, a11, a12, a13],
            [a20, a21, a22, a23],
            [a30, a31, a32, a33],
        ] = self.columns;
        let b00 = a00 * a11 - a01 * a10;
        let b01 = a00 * a12 - a02 * a10;
        let b02 = a00 * a13 - a03 * a10;
        let b03 = a01 * a12 - a02 * a11;
        let b04 = a01 * a13 - a03 * a11;
        let b05 = a02 * a13 - a03 * a12;
        let b06 = a20 * a31 - a21 * a30;
        let b07 = a20 * a32 - a22 * a30;
        let b08 = a20 * a33 - a23 * a30;
        let b09 = a21 * a32 - a22 * a31;
        let b10 = a21 * a33 - a23 * a31;
        let b11 = a22 * a33 - a23 * a32;
        let determinant = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let scale = 1.0 / determinant;
        let inverse = Self::from_columns([
            [
                a11 * b11 - a12 * b10 + a13 * b09,
                a02 * b10 - a01 * b11 - a03 * b09,
                a31 * b05 - a32 * b04 + a33 * b03,
                a22 * b04 - a21 * b05 - a23 * b03,
            ],
            [
                a12 * b08 - a10 * b11 - a13 * b07,
                a00 * b11 - a02 * b08 + a03 * b07,
                a32 * b02 - a30 * b05 - a33 * b01,
                a20 * b05 - a22 * b02 + a23 * b01,
            ],
            [
                a10 * b10 - a11 * b08 + a13 * b06,
                a01 * b08 - a00 * b10 - a03 * b06,
                a30 * b04 - a31 * b02 + a33 * b00,
                a21 * b02 - a20 * b04 - a23 * b00,
            ],
            [
                a11 * b07 - a10 * b09 - a12 * b06,
                a00 * b09 - a01 * b07 + a02 * b06,
                a31 * b01 - a30 * b03 - a32 * b00,
                a20 * b03 - a21 * b01 + a22 * b00,
            ],
        ]);
        let columns = inverse
            .columns
            .map(|column| column.map(|value| value * scale));
        columns
            .as_flattened()
            .iter()
            .all(|value| value.is_finite())
            .then_some(Self::from_columns(columns))
    }

    fn transform(self, vector: [f32; 4]) -> [f32; 4] {
        let mut result = [0.0; 4];
        for (column, component) in self.columns.iter().zip(vector) {
            for (row, value) in result.iter_mut().enumerate() {
                *value += column[row] * component;
            }
        }
        result
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Mat4 {
    type Output = Self;

    /// Combines two transforms: `a * b` applies `b` first, then `a`.
    fn mul(self, other: Self) -> Self {
        Self::from_columns(other.columns.map(|column| self.transform(column)))
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual - expected).length() < EPSILON,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn scale_rotation_translation_apply_in_that_order() {
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 1.0, 1.0),
            Quat::from_rotation_z(FRAC_PI_2),
            Vec3::new(0.0, 0.0, 5.0),
        );

        assert_close(
            transform.transform_point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(0.0, 2.0, 5.0),
        );
        assert_eq!(transform.translation(), Vec3::new(0.0, 0.0, 5.0));
    }

    #[test]
    fn inverses_undo_their_transform() {
        let transform = Mat4::from_translation(Vec3::new(1.0, -2.0, 3.0))
            * Mat4::from_rotation_y(0.7)
            * Mat4::from_rotation_x(-0.3)
            * Mat4::from_scale(Vec3::new(2.0, 0.5, 4.0));
        let point = Vec3::new(0.3, 5.0, -1.0);

        let inverse = transform.inverse().unwrap();

        assert_close(
            inverse.transform_point(transform.transform_point(point)),
            point,
        );
        assert_close(
            transform.transform_point(inverse.transform_point(point)),
            point,
        );
    }

    #[test]
    fn flattening_transforms_have_no_inverse() {
        assert_eq!(Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0)).inverse(), None);
        assert_eq!(
            Mat4::from_translation(Vec3::new(f32::NAN, 0.0, 0.0)).inverse(),
            None
        );
    }

    #[test]
    fn transposing_swaps_rows_and_columns() {
        let translation = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));

        let transposed = translation.transposed();

        assert_eq!(transposed.to_columns()[0], [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(transposed.to_columns()[2], [0.0, 0.0, 1.0, 3.0]);
        assert_eq!(transposed.transposed(), translation);
    }

    #[test]
    fn identity_leaves_points_unchanged() {
        let point = Vec3::new(1.0, 2.0, 3.0);

        assert_close(Mat4::IDENTITY.transform_point(point), point);
    }

    #[test]
    fn translation_moves_points() {
        let matrix = Mat4::from_translation(Vec3::new(1.0, -2.0, 3.0));

        assert_close(
            matrix.transform_point(Vec3::ZERO),
            Vec3::new(1.0, -2.0, 3.0),
        );
    }

    #[test]
    fn rotation_y_turns_x_axis_towards_negative_z() {
        let matrix = Mat4::from_rotation_y(FRAC_PI_2);

        assert_close(
            matrix.transform_point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(0.0, 0.0, -1.0),
        );
    }

    #[test]
    fn rotation_z_turns_x_axis_towards_y() {
        let matrix = Mat4::from_rotation_z(FRAC_PI_2);

        assert_close(
            matrix.transform_point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(0.0, 1.0, 0.0),
        );
    }

    #[test]
    fn multiplication_applies_right_operand_first() {
        let scale = Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0));
        let translation = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));

        let combined = translation * scale;

        assert_close(
            combined.transform_point(Vec3::new(1.0, 0.0, 0.0)),
            Vec3::new(3.0, 0.0, 0.0),
        );
    }

    #[test]
    fn look_at_places_target_in_front_of_camera() {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::UP)
            .expect("valid camera orientation");

        assert_close(view.transform_point(Vec3::ZERO), Vec3::new(0.0, 0.0, -5.0));
    }

    #[test]
    fn look_at_rejects_view_parallel_to_up() {
        assert!(Mat4::look_at(Vec3::ZERO, Vec3::UP, Vec3::UP).is_none());
    }

    #[test]
    fn perspective_maps_near_and_far_planes_to_depth_range() {
        let projection = Mat4::perspective(FRAC_PI_2, 1.0, 0.1, 100.0);

        let near = projection.transform_point(Vec3::new(0.0, 0.0, -0.1));
        let far = projection.transform_point(Vec3::new(0.0, 0.0, -100.0));

        assert!((near.z - 0.0).abs() < EPSILON);
        assert!((far.z - 1.0).abs() < EPSILON);
    }

    #[test]
    fn orthographic_maps_box_corners_to_screen_edges() {
        let projection = Mat4::orthographic(-4.0, 4.0, -2.0, 2.0, 0.0, 10.0);

        assert_close(
            projection.transform_point(Vec3::new(4.0, 2.0, 0.0)),
            Vec3::new(1.0, 1.0, 0.0),
        );
        assert_close(
            projection.transform_point(Vec3::new(-4.0, -2.0, -10.0)),
            Vec3::new(-1.0, -1.0, 1.0),
        );
    }
}
