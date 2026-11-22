//! 4x4 matrix for 3D transforms and projections.

use std::ops::Mul;

use crate::Vec3;

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
