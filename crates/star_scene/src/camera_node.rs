//! Cameras as parts of nodes: what a scene says about the points of view of
//! its game, and their `[node.camera]` and `[node.camera_3d]` tables.

use std::f32::consts::PI;

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};
use star_render::{DEFAULT_2D_VIEW_HEIGHT, DEFAULT_VERTICAL_FOV};

/// A 2D node the game can be seen from: the window is centered on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2dNode {
    /// World units shown from the bottom to the top of the window; the
    /// width follows the window's shape. It must be positive.
    pub view_height: f32,
    /// Whether the game is seen from it as soon as it enters the game.
    pub main: bool,
}

impl Default for Camera2dNode {
    fn default() -> Self {
        Self {
            view_height: DEFAULT_2D_VIEW_HEIGHT,
            main: false,
        }
    }
}

/// A 3D node the game can be seen from, in perspective, looking along its
/// -z axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3dNode {
    /// How wide it sees from the bottom to the top, in radians: more than
    /// 0 and less than a half turn.
    pub fov: f32,
    /// Whether the game is seen from it as soon as it enters the game.
    pub main: bool,
}

impl Default for Camera3dNode {
    fn default() -> Self {
        Self {
            fov: DEFAULT_VERTICAL_FOV,
            main: false,
        }
    }
}

/// A `[node.camera]` table; what it leaves out takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Camera2dData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    view_height: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    main: bool,
}

/// A `[node.camera_3d]` table; what it leaves out takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Camera3dData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fov: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    main: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
const fn is_false(value: &bool) -> bool {
    !*value
}

impl Camera2dData {
    pub(crate) fn to_camera(&self) -> EngineResult<Camera2dNode> {
        let view_height = self.view_height.unwrap_or(DEFAULT_2D_VIEW_HEIGHT);
        if !(view_height.is_finite() && view_height > 0.0) {
            return Err(EngineError::Scene(format!(
                "a camera must show a positive height, not {view_height}"
            )));
        }
        Ok(Camera2dNode {
            view_height,
            main: self.main,
        })
    }

    pub(crate) fn from_camera(camera: Camera2dNode) -> Self {
        Self {
            view_height: (camera.view_height != DEFAULT_2D_VIEW_HEIGHT)
                .then_some(camera.view_height),
            main: camera.main,
        }
    }
}

impl Camera3dData {
    pub(crate) fn to_camera(&self) -> EngineResult<Camera3dNode> {
        let fov = self.fov.unwrap_or(DEFAULT_VERTICAL_FOV);
        if !(fov.is_finite() && fov > 0.0 && fov < PI) {
            return Err(EngineError::Scene(format!(
                "a 3D camera must see more than 0 and less than {PI} radians, not {fov}"
            )));
        }
        Ok(Camera3dNode {
            fov,
            main: self.main,
        })
    }

    pub(crate) fn from_camera(camera: Camera3dNode) -> Self {
        Self {
            fov: (camera.fov != DEFAULT_VERTICAL_FOV).then_some(camera.fov),
            main: camera.main,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_2d(text: &str) -> EngineResult<Camera2dNode> {
        toml::from_str::<Camera2dData>(text).unwrap().to_camera()
    }

    fn camera_3d(text: &str) -> EngineResult<Camera3dNode> {
        toml::from_str::<Camera3dData>(text).unwrap().to_camera()
    }

    #[test]
    fn empty_tables_give_the_default_cameras() {
        assert_eq!(camera_2d("").unwrap(), Camera2dNode::default());
        assert_eq!(camera_3d("").unwrap(), Camera3dNode::default());
    }

    #[test]
    fn tables_give_their_sizes_and_whether_they_are_main() {
        assert_eq!(
            camera_2d("view_height = 10.0\nmain = true").unwrap(),
            Camera2dNode {
                view_height: 10.0,
                main: true
            }
        );
        assert_eq!(camera_3d("fov = 1.2").unwrap().fov, 1.2);
    }

    #[test]
    fn sizes_that_show_nothing_are_scene_errors() {
        for text in [
            "view_height = 0.0",
            "view_height = -3.0",
            "view_height = nan",
        ] {
            assert!(
                matches!(camera_2d(text), Err(EngineError::Scene(_))),
                "{text}"
            );
        }
        for text in ["fov = 0.0", "fov = 3.2", "fov = inf"] {
            assert!(
                matches!(camera_3d(text), Err(EngineError::Scene(_))),
                "{text}"
            );
        }
    }

    #[test]
    fn written_tables_leave_out_the_defaults_and_read_back_the_same() {
        let wide = Camera3dNode {
            fov: 1.5,
            main: true,
        };

        let default_text = toml::to_string(&Camera2dData::from_camera(Camera2dNode::default()));
        let wide_text = toml::to_string(&Camera3dData::from_camera(wide)).unwrap();

        assert_eq!(default_text.unwrap(), "");
        assert_eq!(camera_3d(&wide_text).unwrap(), wide);
    }
}
