//! 2D lights as parts of nodes, and their `[node.light_2d]` table.

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};
use star_math::Vec2;
use star_render::{
    Color, DEFAULT_SPOT_ANGLE, DEFAULT_SPOT_BLEND, Light2d, MAX_SPOT_ANGLE, PointLight2d,
    SpotLight2d,
};

use crate::GlobalTransform2D;

/// Lights the sprites around a 2D node. Sprites are only darkened where no
/// light reaches once the frame's 2D ambient light is darkened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light2dNode {
    /// Light from the node's position, in every direction.
    Point {
        /// Its color.
        color: Color,
        /// How bright it is where the node is.
        intensity: f32,
        /// Beyond this distance it lights nothing.
        range: f32,
    },
    /// Light from the node's position, in a cone along its +x axis.
    Spot {
        /// Its color.
        color: Color,
        /// How bright it is where the node is.
        intensity: f32,
        /// Beyond this distance it lights nothing.
        range: f32,
        /// From the cone's axis to its edge, in radians.
        angle: f32,
        /// How much of the cone, from its edge inwards, fades: 0 to 1.
        blend: f32,
    },
}

impl Light2dNode {
    /// The light shining from a node placed at `global`.
    #[must_use]
    pub fn to_light(self, global: GlobalTransform2D) -> Light2d {
        let position = global.position();
        match self {
            Self::Point {
                color,
                intensity,
                range,
            } => PointLight2d {
                position,
                color,
                intensity,
                range,
            }
            .into(),
            Self::Spot {
                color,
                intensity,
                range,
                angle,
                blend,
            } => {
                let (sin, cos) = global.rotation().sin_cos();
                SpotLight2d {
                    position,
                    direction: Vec2::new(cos, sin),
                    color,
                    intensity,
                    range,
                    angle,
                    blend,
                }
                .into()
            }
        }
    }
}

/// A `[node.light_2d]` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Light2dData {
    kind: Light2dKindData,
    /// Red, green and blue, from 0 to 1, in sRGB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intensity: Option<f32>,
    range: f32,
    /// Of spots: from the cone's axis to its edge, in radians.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    angle: Option<f32>,
    /// Of spots: how much of the cone fades, from 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blend: Option<f32>,
}

/// What a 2D light is, as written in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Light2dKindData {
    Point,
    Spot,
}

/// The intensity of a light that gives none.
const DEFAULT_INTENSITY: f32 = 1.0;

impl Light2dData {
    pub(crate) fn to_light(&self) -> EngineResult<Light2dNode> {
        let color = self
            .color
            .map_or(Color::WHITE, |[r, g, b]| Color::rgb(r, g, b));
        let intensity = self.intensity.unwrap_or(DEFAULT_INTENSITY);
        let range = self.range;
        if !(range.is_finite() && range > 0.0) {
            return Err(EngineError::Scene(format!(
                "a 2D light needs a positive range, not {range}"
            )));
        }
        if self.kind == Light2dKindData::Point {
            if self.angle.is_some() || self.blend.is_some() {
                return Err(EngineError::Scene(
                    "only spot lights have an angle and a blend".to_owned(),
                ));
            }
            return Ok(Light2dNode::Point {
                color,
                intensity,
                range,
            });
        }
        let angle = self.angle.unwrap_or(DEFAULT_SPOT_ANGLE);
        let blend = self.blend.unwrap_or(DEFAULT_SPOT_BLEND);
        if !(angle > 0.0 && angle < MAX_SPOT_ANGLE) {
            return Err(EngineError::Scene(format!(
                "a spot light's angle is more than 0 and less than {MAX_SPOT_ANGLE} radians, \
                 not {angle}"
            )));
        }
        if !(0.0..=1.0).contains(&blend) {
            return Err(EngineError::Scene(format!(
                "a spot light's blend goes from 0 to 1, not {blend}"
            )));
        }
        Ok(Light2dNode::Spot {
            color,
            intensity,
            range,
            angle,
            blend,
        })
    }

    pub(crate) fn from_light(light: Light2dNode) -> Self {
        let (kind, color, intensity, range, cone) = match light {
            Light2dNode::Point {
                color,
                intensity,
                range,
            } => (Light2dKindData::Point, color, intensity, range, None),
            Light2dNode::Spot {
                color,
                intensity,
                range,
                angle,
                blend,
            } => (
                Light2dKindData::Spot,
                color,
                intensity,
                range,
                Some((angle, blend)),
            ),
        };
        Self {
            kind,
            color: (color != Color::WHITE).then_some([color.r, color.g, color.b]),
            intensity: (intensity != DEFAULT_INTENSITY).then_some(intensity),
            range,
            angle: cone.map(|(angle, _)| angle),
            blend: cone.map(|(_, blend)| blend),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use star_math::Affine2;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn read(text: &str) -> EngineResult<Light2dNode> {
        toml::from_str::<Light2dData>(text)
            .map_err(|error| EngineError::Scene(error.to_string()))?
            .to_light()
    }

    #[test]
    fn tables_give_lights_that_read_back_the_same() {
        let lamp = read("kind = \"point\"\nrange = 3.0\ncolor = [1.0, 0.5, 0.0]").unwrap();
        let spot = read("kind = \"spot\"\nrange = 5.0\nangle = 0.4").unwrap();

        assert!(matches!(lamp, Light2dNode::Point { range: 3.0, .. }));
        assert!(matches!(
            spot,
            Light2dNode::Spot {
                angle: 0.4,
                blend: DEFAULT_SPOT_BLEND,
                ..
            }
        ));
        for light in [lamp, spot] {
            let text = toml::to_string(&Light2dData::from_light(light)).unwrap();
            assert_eq!(read(&text).unwrap(), light);
        }
    }

    #[test]
    fn lights_that_cannot_shine_are_scene_errors() {
        for broken in [
            "kind = \"point\"",
            "kind = \"point\"\nrange = 0.0",
            "kind = \"point\"\nrange = 1.0\nangle = 0.5",
            "kind = \"spot\"\nrange = 1.0\nangle = 2.0",
            "kind = \"spot\"\nrange = 1.0\nblend = -1.0",
        ] {
            assert!(read(broken).is_err(), "{broken}");
        }
    }

    #[test]
    fn spots_shine_along_the_turned_x_axis_of_their_node() {
        let spot = Light2dNode::Spot {
            color: Color::WHITE,
            intensity: 1.0,
            range: 2.0,
            angle: 0.5,
            blend: 0.2,
        };
        let global = GlobalTransform2D::from_affine(Affine2::from_scale_rotation_translation(
            Vec2::ONE,
            FRAC_PI_2,
            Vec2::new(3.0, 1.0),
        ));

        let Light2d::Spot(light) = spot.to_light(global) else {
            panic!("a spot");
        };

        assert!((light.position - Vec2::new(3.0, 1.0)).length() < EPSILON);
        assert!((light.direction - Vec2::new(0.0, 1.0)).length() < EPSILON);
    }
}
