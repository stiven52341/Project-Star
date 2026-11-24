//! Point of view from which a frame is drawn.

use std::f32::consts::FRAC_PI_3;

use star_math::{Mat4, Vec2, Vec3};
use star_platform::WindowSize;

/// Closest distance to the camera that is still drawn.
const NEAR_PLANE: f32 = 0.1;
/// Farthest distance from the camera that is still drawn.
const FAR_PLANE: f32 = 1000.0;
/// How far a 2D camera sits in front of the `z = 0` plane it looks at.
const CAMERA_2D_DISTANCE: f32 = 10.0;
/// World units visible from the bottom to the top of the screen by default.
pub const DEFAULT_2D_VIEW_HEIGHT: f32 = 2.0;
/// 60 degrees: a natural-looking default field of view.
pub const DEFAULT_VERTICAL_FOV: f32 = FRAC_PI_3;

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

    /// 2D camera centered on `center` of the `z = 0` plane, showing
    /// `view_height` world units vertically, as [`Camera::new_2d`].
    #[must_use]
    pub const fn centered_2d(center: Vec2, view_height: f32) -> Self {
        let mut camera = Self::new_2d(view_height);
        camera.position.x = center.x;
        camera.position.y = center.y;
        camera.target.x = center.x;
        camera.target.y = center.y;
        camera
    }

    /// 3D perspective camera placed and turned by `transform`, looking
    /// along its -Z axis with `vertical_fov` radians. Its roll is ignored:
    /// +Y stays up, so the view never tilts.
    #[must_use]
    pub fn along_transform(transform: Mat4, vertical_fov: f32) -> Self {
        let position = transform.translation();
        let [_, _, backward, _] = transform.to_columns();
        let forward = Vec3::new(-backward[0], -backward[1], -backward[2]);
        Self {
            position,
            target: position + forward,
            projection: Projection::Perspective { vertical_fov },
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

    /// The point of the `z = 0` plane seen at `pixel` of a window of size
    /// `window` (physical pixels from its top-left corner, Y downwards), for
    /// a 2D camera: orthographic and looking straight along -Z, as
    /// [`Camera::new_2d`] makes them. `None` for perspective cameras and
    /// empty windows.
    #[must_use]
    // Pixel dimensions are far below 2^24, so converting them to `f32` is exact.
    #[allow(clippy::cast_precision_loss)]
    pub fn screen_to_world_2d(&self, pixel: Vec2, window: WindowSize) -> Option<Vec2> {
        let Projection::Orthographic { view_height } = self.projection else {
            return None;
        };
        if window.is_empty() {
            return None;
        }
        let width = window.width as f32;
        let height = window.height as f32;
        let units_per_pixel = view_height / height;
        Some(Vec2::new(
            self.position.x + (pixel.x - width / 2.0) * units_per_pixel,
            self.position.y - (pixel.y - height / 2.0) * units_per_pixel,
        ))
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

    #[test]
    fn cameras_follow_the_place_they_are_given() {
        let flat = Camera::centered_2d(Vec2::new(3.0, -1.0), 5.0);
        let turned = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let solid = Camera::along_transform(turned, 0.5);

        assert_eq!(flat.target, Vec3::new(3.0, -1.0, 0.0));
        assert_eq!(
            flat.projection,
            Projection::Orthographic { view_height: 5.0 }
        );
        // A quarter turn about +Y points -Z (forward) to -X.
        assert!((solid.target - Vec3::new(0.0, 2.0, 3.0)).length() < 1e-5);
        assert_eq!(
            solid.projection,
            Projection::Perspective { vertical_fov: 0.5 }
        );
    }

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
    fn screen_points_are_found_in_the_2d_world() {
        let mut camera = Camera::new_2d(2.0);
        camera.position.x = 5.0;
        camera.target.x = 5.0;
        let window = WindowSize::new(200, 100);

        let center = camera.screen_to_world_2d(Vec2::new(100.0, 50.0), window);
        let top_left = camera.screen_to_world_2d(Vec2::ZERO, window);

        assert_eq!(center, Some(Vec2::new(5.0, 0.0)));
        assert_eq!(top_left, Some(Vec2::new(3.0, 1.0)));
    }

    #[test]
    fn only_2d_cameras_on_a_window_find_screen_points() {
        let perspective = Camera::new_3d(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let flat = Camera::new_2d(2.0);

        assert_eq!(
            perspective.screen_to_world_2d(Vec2::ZERO, WindowSize::new(10, 10)),
            None
        );
        assert_eq!(
            flat.screen_to_world_2d(Vec2::ZERO, WindowSize::new(10, 0)),
            None
        );
    }

    #[test]
    fn camera_at_its_target_has_no_view() {
        let camera = Camera::new_3d(Vec3::ZERO, Vec3::ZERO);

        assert!(camera.view_projection(1.0).is_none());
    }
}
