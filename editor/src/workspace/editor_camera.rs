//! Camera the user moves around the scene viewport.

use std::f32::consts::FRAC_PI_2;

use star_engine::prelude::{Camera, Projection, Vec2, Vec3};

/// World units visible vertically when a 2D view opens.
const DEFAULT_2D_VIEW_HEIGHT: f32 = 12.0;
const MIN_2D_VIEW_HEIGHT: f32 = 1.0;
const MAX_2D_VIEW_HEIGHT: f32 = 500.0;
/// Distance between a 2D camera and the `z = 0` plane.
const CAMERA_2D_DISTANCE: f32 = 10.0;

const DEFAULT_3D_YAW: f32 = 0.6;
const DEFAULT_3D_PITCH: f32 = 0.5;
const DEFAULT_3D_DISTANCE: f32 = 14.0;
const MIN_3D_DISTANCE: f32 = 1.0;
const MAX_3D_DISTANCE: f32 = 500.0;
/// Keeps the camera from looking straight down or up, where "up" is undefined.
const MAX_PITCH: f32 = FRAC_PI_2 - 0.05;
/// Radians turned per pixel dragged.
const ORBIT_SPEED: f32 = 0.008;
/// Zoom change per unit of mouse wheel scroll.
const ZOOM_SPEED: f32 = 0.002;
const VERTICAL_FOV: f32 = std::f32::consts::FRAC_PI_3;

/// Viewport camera: pans over a plane in 2D, orbits around a point in 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EditorCamera {
    TwoD { center: Vec2, view_height: f32 },
    ThreeD { yaw: f32, pitch: f32, distance: f32 },
}

impl EditorCamera {
    pub(crate) const fn new_2d() -> Self {
        Self::TwoD {
            center: Vec2::new(0.0, 0.0),
            view_height: DEFAULT_2D_VIEW_HEIGHT,
        }
    }

    pub(crate) const fn new_3d() -> Self {
        Self::ThreeD {
            yaw: DEFAULT_3D_YAW,
            pitch: DEFAULT_3D_PITCH,
            distance: DEFAULT_3D_DISTANCE,
        }
    }

    /// Reacts to the mouse being dragged by `pixels` over a viewport
    /// `viewport_height` pixels tall: pans in 2D, orbits in 3D.
    pub(crate) fn drag(&mut self, pixels: Vec2, viewport_height: f32) {
        match self {
            Self::TwoD {
                center,
                view_height,
            } => {
                // Moves the world with the pointer: dragging right shows what is on the left.
                let units_per_pixel = *view_height / viewport_height.max(1.0);
                center.x -= pixels.x * units_per_pixel;
                center.y += pixels.y * units_per_pixel;
            }
            Self::ThreeD { yaw, pitch, .. } => {
                *yaw -= pixels.x * ORBIT_SPEED;
                *pitch = (*pitch + pixels.y * ORBIT_SPEED).clamp(-MAX_PITCH, MAX_PITCH);
            }
        }
    }

    /// Zooms in for positive `scroll` (wheel forward) and out for negative.
    pub(crate) fn zoom(&mut self, scroll: f32) {
        let factor = (-scroll * ZOOM_SPEED).exp();
        match self {
            Self::TwoD { view_height, .. } => {
                *view_height =
                    (*view_height * factor).clamp(MIN_2D_VIEW_HEIGHT, MAX_2D_VIEW_HEIGHT);
            }
            Self::ThreeD { distance, .. } => {
                *distance = (*distance * factor).clamp(MIN_3D_DISTANCE, MAX_3D_DISTANCE);
            }
        }
    }

    /// Engine camera matching this viewpoint.
    pub(crate) fn to_camera(self) -> Camera {
        match self {
            Self::TwoD {
                center,
                view_height,
            } => Camera {
                position: Vec3::new(center.x, center.y, CAMERA_2D_DISTANCE),
                target: Vec3::new(center.x, center.y, 0.0),
                projection: Projection::Orthographic { view_height },
            },
            Self::ThreeD {
                yaw,
                pitch,
                distance,
            } => Camera {
                position: orbit_position(yaw, pitch, distance),
                target: Vec3::ZERO,
                projection: Projection::Perspective {
                    vertical_fov: VERTICAL_FOV,
                },
            },
        }
    }
}

/// Point on a sphere of radius `distance` around the origin.
fn orbit_position(yaw: f32, pitch: f32, distance: f32) -> Vec3 {
    let (yaw_sin, yaw_cos) = yaw.sin_cos();
    let (pitch_sin, pitch_cos) = pitch.sin_cos();
    Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos) * distance
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn dragging_a_2d_view_moves_the_world_with_the_pointer() {
        let mut camera = EditorCamera::new_2d();

        camera.drag(Vec2::new(100.0, 0.0), 100.0);

        let Camera { target, .. } = camera.to_camera();
        assert!((target.x + DEFAULT_2D_VIEW_HEIGHT).abs() < EPSILON);
    }

    #[test]
    fn zooming_in_shows_less_of_a_2d_world() {
        let mut camera = EditorCamera::new_2d();

        camera.zoom(100.0);

        let EditorCamera::TwoD { view_height, .. } = camera else {
            panic!("still a 2D camera");
        };
        assert!(view_height < DEFAULT_2D_VIEW_HEIGHT);
    }

    #[test]
    fn orbiting_keeps_the_distance_to_the_target() {
        let mut camera = EditorCamera::new_3d();

        camera.drag(Vec2::new(250.0, -80.0), 600.0);

        let Camera {
            position, target, ..
        } = camera.to_camera();
        assert!(((position - target).length() - DEFAULT_3D_DISTANCE).abs() < EPSILON);
    }

    #[test]
    fn pitch_never_reaches_the_poles() {
        let mut camera = EditorCamera::new_3d();

        camera.drag(Vec2::new(0.0, 100_000.0), 600.0);

        assert!(camera.to_camera().view_projection(1.0).is_some());
    }

    #[test]
    fn zoom_is_clamped() {
        let mut camera = EditorCamera::new_3d();

        camera.zoom(1_000_000.0);

        let EditorCamera::ThreeD { distance, .. } = camera else {
            panic!("still a 3D camera");
        };
        assert!((distance - MIN_3D_DISTANCE).abs() < EPSILON);
    }
}
