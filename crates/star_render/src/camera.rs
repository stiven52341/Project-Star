//! Point of view from which a frame is drawn.

use std::f32::consts::FRAC_PI_3;

use star_math::{Mat4, Vec3};

/// Closest distance to the camera that is still drawn.
const NEAR_PLANE: f32 = 0.1;
/// Farthest distance from the camera that is still drawn.
const FAR_PLANE: f32 = 1000.0;
/// How far a 2D camera sits in front of the `z = 0` plane it looks at.
const CAMERA_2D_DISTANCE: f32 = 10.0;
/// World units visible from the bottom to the top of the screen by default.
const DEFAULT_2D_VIEW_HEIGHT: f32 = 2.0;
/// 60 degrees: a natural-looking default field of view.
const DEFAULT_VERTICAL_FOV: f32 = FRAC_PI_3;

/// How the camera flattens the 3D world onto the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Objects keep their size at any distance. Used for 2D games.
    Orthographic {
        /// World units visible from the bottom to the top of the screen.
        view_height: f32,
    },
    /// Distant objects look smaller. Used for 3D games.
    Perspective {
        /// Vertical field of view, in radians.
        vertical_fov: f32,
    },
}

/// A camera looking from `position` towards `target`, with +Y as up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Where the camera is.
    pub position: Vec3,
    /// Point the camera looks at.
    pub target: Vec3,
    /// How the scene is projected onto the screen.
    pub projection: Projection,
}

impl Camera {
    /// 2D camera looking at the `z = 0` plane, centered on the origin, that
    /// shows `view_height` world units vertically. The visible width follows
    /// the window's aspect ratio, so shapes never look stretched.
    #[must_use]
    pub const fn new_2d(view_height: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, CAMERA_2D_DISTANCE),
            target: Vec3::ZERO,
            projection: Projection::Orthographic { view_height },
        }
    }

    /// 3D perspective camera at `position` looking at `target`.
    #[must_use]
    pub const fn new_3d(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            projection: Projection::Perspective {
                vertical_fov: DEFAULT_VERTICAL_FOV,
            },
        }
    }

    /// Matrix that takes world coordinates to the screen.
    ///
    /// Returns `None` if `position` equals `target` or the camera looks
    /// straight up or down, since its orientation would be undefined.
    #[must_use]
    pub fn view_projection(&self, aspect_ratio: f32) -> Option<Mat4> {
        let view = Mat4::look_at(self.position, self.target, Vec3::UP)?;
        Some(self.projection_matrix(aspect_ratio) * view)
    }

    fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        match self.projection {
            Projection::Orthographic { view_height } => {
                let half_height = view_height / 2.0;
                let half_width = half_height * aspect_ratio;
                Mat4::orthographic(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    NEAR_PLANE,
                    FAR_PLANE,
                )
            }
            Projection::Perspective { vertical_fov } => {
                Mat4::perspective(vertical_fov, aspect_ratio, NEAR_PLANE, FAR_PLANE)
            }
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new_2d(DEFAULT_2D_VIEW_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn camera_2d_maps_view_height_to_screen_edges() {
        let matrix = Camera::new_2d(4.0)
            .view_projection(1.0)
            .expect("valid camera");

        let top = matrix.transform_point(Vec3::new(0.0, 2.0, 0.0));

        assert!((top.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn camera_2d_widens_view_with_aspect_ratio() {
        let matrix = Camera::new_2d(2.0)
            .view_projection(2.0)
            .expect("valid camera");

        let right_edge = matrix.transform_point(Vec3::new(2.0, 0.0, 0.0));

        assert!((right_edge.x - 1.0).abs() < EPSILON);
    }

    #[test]
    fn camera_3d_shows_target_at_screen_center() {
        let camera = Camera::new_3d(Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO);
        let matrix = camera.view_projection(1.5).expect("valid camera");

        let center = matrix.transform_point(Vec3::ZERO);

        assert!(center.x.abs() < EPSILON && center.y.abs() < EPSILON);
    }

    #[test]
    fn camera_at_its_target_has_no_view() {
        let camera = Camera::new_3d(Vec3::ZERO, Vec3::ZERO);

        assert!(camera.view_projection(1.0).is_none());
    }
}
