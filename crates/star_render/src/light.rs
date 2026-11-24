//! Lights of the 3D meshes.
//!
//! A light of intensity 1 shining straight on a white matte surface makes
//! it white. Colors are in sRGB, like every other color of the engine.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

use star_math::Vec3;

use crate::Color;
use crate::bytes::extend_with_f32s;

/// The most lights a frame can have, besides its ambient light.
pub const MAX_LIGHTS: usize = 16;

/// Light that reaches every surface from everywhere, so the sides no light
/// faces are not black.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientLight {
    /// Its color.
    pub color: Color,
    /// How bright it is: 0 is no light.
    pub intensity: f32,
}

impl AmbientLight {
    /// No ambient light: what lights do not reach is black.
    pub const NONE: Self = Self {
        color: Color::WHITE,
        intensity: 0.0,
    };

    /// White light at full intensity: surfaces show their own colors. The
    /// 2D ambient light of a frame, unless the game changes it.
    pub const FULL: Self = Self {
        color: Color::WHITE,
        intensity: 1.0,
    };

    /// An ambient light of `color` and `intensity`.
    #[must_use]
    pub const fn new(color: Color, intensity: f32) -> Self {
        Self { color, intensity }
    }
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self::NONE
    }
}

/// Light from very far away, all in the same direction: the sun.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    /// Where the light goes, e.g. down for a sun at noon. A zero direction
    /// lights nothing.
    pub direction: Vec3,
    /// Its color.
    pub color: Color,
    /// How bright it is.
    pub intensity: f32,
}

/// Light from a point, in every direction, fading with distance: a lamp, a
/// fire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    /// Where it is, in the world.
    pub position: Vec3,
    /// Its color.
    pub color: Color,
    /// How bright it is one world unit away.
    pub intensity: f32,
    /// Beyond this distance it lights nothing; it must be positive.
    pub range: f32,
}

/// How wide spot lights shine when nothing says otherwise: 30° from their
/// axis to the edge of their cone.
pub const DEFAULT_SPOT_ANGLE: f32 = FRAC_PI_6;
/// How much of a spot light's cone fades into the dark when nothing says
/// otherwise: its outer fifth.
pub const DEFAULT_SPOT_BLEND: f32 = 0.2;
/// The widest a spot light can shine, from its axis to its edge: less than
/// a right angle, or it would no longer be a cone.
pub const MAX_SPOT_ANGLE: f32 = FRAC_PI_2;

/// Light from a point, in a cone, fading with distance: a flashlight, a
/// stage light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotLight {
    /// Where it is, in the world.
    pub position: Vec3,
    /// Where its cone points. A zero direction lights nothing.
    pub direction: Vec3,
    /// Its color.
    pub color: Color,
    /// How bright it is one world unit away.
    pub intensity: f32,
    /// Beyond this distance it lights nothing; it must be positive.
    pub range: f32,
    /// From the cone's axis to its edge, in radians: more than 0 and less
    /// than [`MAX_SPOT_ANGLE`].
    pub angle: f32,
    /// How much of the cone, from its edge inwards, fades into the dark:
    /// 0 is a sharp edge, 1 fades from the very axis.
    pub blend: f32,
}

/// A light of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    /// See [`DirectionalLight`].
    Directional(DirectionalLight),
    /// See [`PointLight`].
    Point(PointLight),
    /// See [`SpotLight`].
    Spot(SpotLight),
}

impl From<SpotLight> for Light {
    fn from(light: SpotLight) -> Self {
        Self::Spot(light)
    }
}

impl From<DirectionalLight> for Light {
    fn from(light: DirectionalLight) -> Self {
        Self::Directional(light)
    }
}

impl From<PointLight> for Light {
    fn from(light: PointLight) -> Self {
        Self::Point(light)
    }
}

/// Size of the `Lights` struct of `surface.wgsl`: the camera's position,
/// the ambient light, the count and [`MAX_LIGHTS`] lights of four
/// `vec4<f32>` each.
pub(crate) const LIGHTS_SIZE: usize = 16 * 3 + MAX_LIGHTS * GPU_LIGHT_SIZE;
const GPU_LIGHT_SIZE: usize = 64;
/// The `w` of a light's position: a direction, a point or a spot.
const DIRECTION: f32 = 0.0;
const POINT: f32 = 1.0;
const SPOT: f32 = 2.0;

/// Writes the lights of a frame as `surface.wgsl` reads them, with their
/// colors already in the target's space (`to_target_space`). Lights that
/// cannot shine (no direction, no range, numbers that are not finite) are
/// left out. `lights` holds at most [`MAX_LIGHTS`].
pub(crate) fn encode_lights(
    bytes: &mut Vec<u8>,
    camera_position: Vec3,
    ambient: AmbientLight,
    lights: &[Light],
    to_target_space: impl Fn(Color) -> Color,
) {
    let radiance = |color: Color, intensity: f32| {
        let Color { r, g, b, .. } = to_target_space(color);
        [r * intensity, g * intensity, b * intensity]
    };
    let Vec3 { x, y, z } = camera_position;
    extend_with_f32s(bytes, &[x, y, z, 0.0]);
    let [r, g, b] = radiance(ambient.color, ambient.intensity.max(0.0));
    extend_with_f32s(bytes, &[r, g, b, 0.0]);
    let count_at = bytes.len();
    bytes.extend_from_slice(&[0; 16]);
    let mut count: u32 = 0;
    for light in lights.iter().take(MAX_LIGHTS) {
        if let Some(values) = gpu_light(light, radiance) {
            extend_with_f32s(bytes, &values);
            count += 1;
        }
    }
    bytes[count_at..count_at + 4].copy_from_slice(&count.to_le_bytes());
    let missing = MAX_LIGHTS - count as usize;
    bytes.resize(bytes.len() + missing * GPU_LIGHT_SIZE, 0);
}

/// A light's position (or direction) and radiance (and range); and for
/// spots, where they point and the cosines of the edges of their cone.
fn gpu_light(light: &Light, radiance: impl Fn(Color, f32) -> [f32; 3]) -> Option<[f32; 16]> {
    let reaches = |range: f32| range.is_finite() && range > 0.0;
    let (place, kind, color, intensity, range) = match *light {
        Light::Directional(sun) => (
            sun.direction.normalized()?,
            DIRECTION,
            sun.color,
            sun.intensity,
            0.0,
        ),
        Light::Point(lamp) => {
            if !reaches(lamp.range) {
                return None;
            }
            (lamp.position, POINT, lamp.color, lamp.intensity, lamp.range)
        }
        Light::Spot(spot) => {
            if !reaches(spot.range) {
                return None;
            }
            (spot.position, SPOT, spot.color, spot.intensity, spot.range)
        }
    };
    let (axis, [outer, inner]) = match *light {
        Light::Spot(spot) => (spot.direction.normalized()?, cone_cosines(spot)?),
        _ => (Vec3::ZERO, [0.0, 0.0]),
    };
    let [r, g, b] = radiance(color, intensity.max(0.0));
    let values = [
        place.x, place.y, place.z, kind, r, g, b, range, axis.x, axis.y, axis.z, 0.0, outer, inner,
        0.0, 0.0,
    ];
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(values)
}

/// The cosines of the angles of the edge of a spot's cone and of where it
/// starts to fade; `None` for a cone that is no cone.
fn cone_cosines(spot: SpotLight) -> Option<[f32; 2]> {
    if !(spot.angle > 0.0 && spot.angle < MAX_SPOT_ANGLE) {
        return None;
    }
    let inner = spot.angle * (1.0 - spot.blend.clamp(0.0, 1.0));
    Some([spot.angle.cos(), inner.cos()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(lights: &[Light]) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_lights(
            &mut bytes,
            Vec3::ZERO,
            AmbientLight::new(Color::WHITE, 0.5),
            lights,
            |color| color,
        );
        bytes
    }

    fn count(bytes: &[u8]) -> u32 {
        u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]])
    }

    fn sun(direction: Vec3) -> Light {
        DirectionalLight {
            direction,
            color: Color::WHITE,
            intensity: 1.0,
        }
        .into()
    }

    #[test]
    fn lights_fill_the_whole_struct() {
        let bytes = encoded(&[sun(Vec3::new(0.0, -2.0, 0.0))]);

        assert_eq!(bytes.len(), LIGHTS_SIZE);
        assert_eq!(count(&bytes), 1);
        // The direction is normalized.
        assert_eq!(&bytes[48..56], &[0, 0, 0, 0, 0, 0, 128, 191]);
    }

    #[test]
    fn lights_that_cannot_shine_are_left_out() {
        let lamp = |range: f32| -> Light {
            PointLight {
                position: Vec3::ZERO,
                color: Color::WHITE,
                intensity: 1.0,
                range,
            }
            .into()
        };

        let bytes = encoded(&[sun(Vec3::ZERO), lamp(0.0), lamp(f32::NAN), lamp(3.0)]);

        assert_eq!(count(&bytes), 1);
        assert_eq!(bytes.len(), LIGHTS_SIZE);
    }

    fn spot(angle: f32, blend: f32) -> SpotLight {
        SpotLight {
            position: Vec3::ZERO,
            direction: Vec3::new(0.0, 0.0, -2.0),
            color: Color::WHITE,
            intensity: 1.0,
            range: 5.0,
            angle,
            blend,
        }
    }

    #[test]
    fn spots_give_the_cosines_of_their_cone() {
        let [outer, inner] = cone_cosines(spot(FRAC_PI_6, 0.5)).unwrap();

        assert!((outer - FRAC_PI_6.cos()).abs() < 1e-6);
        assert!((inner - (FRAC_PI_6 / 2.0).cos()).abs() < 1e-6);
        let [_, sharp] = cone_cosines(spot(FRAC_PI_6, -3.0)).unwrap();
        assert!(
            (sharp - outer).abs() < 1e-6,
            "the blend is kept from 0 to 1"
        );
    }

    #[test]
    fn spots_that_are_no_cone_are_left_out() {
        let lights: Vec<Light> = [0.0, MAX_SPOT_ANGLE, f32::NAN, DEFAULT_SPOT_ANGLE]
            .map(|angle| spot(angle, DEFAULT_SPOT_BLEND).into())
            .to_vec();

        let bytes = encoded(&lights);

        assert_eq!(count(&bytes), 1);
        // The spot's axis is normalized, after its position and radiance.
        let axis_z = 48 + 4 * 10;
        assert_eq!(&bytes[axis_z..axis_z + 4], &(-1.0f32).to_le_bytes());
    }
}
