//! Lights of the sprites.
//!
//! Sprites are lit by the frame's 2D ambient light, which is white at full
//! intensity unless the game changes it, so sprites show their own colors
//! until a game darkens the ambient light and lights them. A 2D light adds
//! its color to the sprites it reaches, fading smoothly to nothing at its
//! range.

use star_math::Vec2;

use crate::bytes::extend_with_f32s;
use crate::light::MAX_SPOT_ANGLE;
use crate::{AmbientLight, Color, MAX_LIGHTS};

/// Light from a point of the 2D world, in every direction, fading to
/// nothing at its range: a torch, a lamp seen from above.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight2d {
    /// Where it is, in the world.
    pub position: Vec2,
    /// Its color.
    pub color: Color,
    /// How bright it is where it is: 1 lights a sprite with its own colors.
    pub intensity: f32,
    /// Beyond this distance it lights nothing; it must be positive.
    pub range: f32,
}

/// Light from a point of the 2D world, in a cone: a flashlight seen from
/// above.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotLight2d {
    /// Where it is, in the world.
    pub position: Vec2,
    /// Where its cone points. A zero direction lights nothing.
    pub direction: Vec2,
    /// Its color.
    pub color: Color,
    /// How bright it is where it is.
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

/// A light of the sprites of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light2d {
    /// See [`PointLight2d`].
    Point(PointLight2d),
    /// See [`SpotLight2d`].
    Spot(SpotLight2d),
}

impl From<PointLight2d> for Light2d {
    fn from(light: PointLight2d) -> Self {
        Self::Point(light)
    }
}

impl From<SpotLight2d> for Light2d {
    fn from(light: SpotLight2d) -> Self {
        Self::Spot(light)
    }
}

/// Size of the `Lights` struct of `sprite.wgsl`: the ambient light, the
/// count and [`MAX_LIGHTS`] lights of three `vec4<f32>` each.
pub(crate) const LIGHTS_2D_SIZE: usize = 16 * 2 + MAX_LIGHTS * GPU_LIGHT_2D_SIZE;
const GPU_LIGHT_2D_SIZE: usize = 48;
/// The `w` of a light's place: a point or a spot.
const POINT: f32 = 1.0;
const SPOT: f32 = 2.0;

/// Writes the 2D lights of a frame as `sprite.wgsl` reads them, with their
/// colors already in the target's space (`to_target_space`). Lights that
/// cannot shine (no range, a cone that is no cone, numbers that are not
/// finite) are left out. `lights` holds at most [`MAX_LIGHTS`].
pub(crate) fn encode_lights_2d(
    bytes: &mut Vec<u8>,
    ambient: AmbientLight,
    lights: &[Light2d],
    to_target_space: impl Fn(Color) -> Color,
) {
    let radiance = |color: Color, intensity: f32| {
        let Color { r, g, b, .. } = to_target_space(color);
        [r * intensity, g * intensity, b * intensity]
    };
    let [r, g, b] = radiance(ambient.color, ambient.intensity.max(0.0));
    extend_with_f32s(bytes, &[r, g, b, 0.0]);
    let count_at = bytes.len();
    bytes.extend_from_slice(&[0; 16]);
    let mut count: u32 = 0;
    for light in lights.iter().take(MAX_LIGHTS) {
        if let Some(values) = gpu_light_2d(light, radiance) {
            extend_with_f32s(bytes, &values);
            count += 1;
        }
    }
    bytes[count_at..count_at + 4].copy_from_slice(&count.to_le_bytes());
    let missing = MAX_LIGHTS - count as usize;
    bytes.resize(bytes.len() + missing * GPU_LIGHT_2D_SIZE, 0);
}

/// A light's place, range and kind; its radiance; and for spots, where
/// they point and the cosines of the edges of their cone.
fn gpu_light_2d(light: &Light2d, radiance: impl Fn(Color, f32) -> [f32; 3]) -> Option<[f32; 12]> {
    let (position, kind, color, intensity, range, cone) = match *light {
        Light2d::Point(lamp) => (
            lamp.position,
            POINT,
            lamp.color,
            lamp.intensity,
            lamp.range,
            [0.0; 4],
        ),
        Light2d::Spot(spot) => (
            spot.position,
            SPOT,
            spot.color,
            spot.intensity,
            spot.range,
            spot_cone(&spot)?,
        ),
    };
    if !(range.is_finite() && range > 0.0) {
        return None;
    }
    let [r, g, b] = radiance(color, intensity.max(0.0));
    let [axis_x, axis_y, outer, inner] = cone;
    let values = [
        position.x, position.y, range, kind, r, g, b, 0.0, axis_x, axis_y, outer, inner,
    ];
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(values)
}

/// A spot's axis and the cosines of the angles of the edge of its cone and
/// of where it starts to fade; `None` for a cone that is no cone.
fn spot_cone(spot: &SpotLight2d) -> Option<[f32; 4]> {
    let axis = spot.direction.normalized()?;
    if !(spot.angle > 0.0 && spot.angle < MAX_SPOT_ANGLE) {
        return None;
    }
    let inner = spot.angle * (1.0 - spot.blend.clamp(0.0, 1.0));
    Some([axis.x, axis.y, spot.angle.cos(), inner.cos()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_SPOT_ANGLE;

    fn encoded(lights: &[Light2d]) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_lights_2d(&mut bytes, AmbientLight::FULL, lights, |color| color);
        bytes
    }

    fn count(bytes: &[u8]) -> u32 {
        u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]])
    }

    fn lamp(range: f32) -> Light2d {
        PointLight2d {
            position: Vec2::new(1.0, 2.0),
            color: Color::WHITE,
            intensity: 1.0,
            range,
        }
        .into()
    }

    fn spot(direction: Vec2, angle: f32) -> Light2d {
        SpotLight2d {
            position: Vec2::ZERO,
            direction,
            color: Color::WHITE,
            intensity: 1.0,
            range: 3.0,
            angle,
            blend: 0.0,
        }
        .into()
    }

    #[test]
    fn lights_fill_the_whole_struct() {
        let bytes = encoded(&[lamp(4.0)]);

        assert_eq!(bytes.len(), LIGHTS_2D_SIZE);
        assert_eq!(count(&bytes), 1);
        assert_eq!(&bytes[32..36], &1.0f32.to_le_bytes());
        assert_eq!(&bytes[40..44], &4.0f32.to_le_bytes());
    }

    #[test]
    fn lights_that_cannot_shine_are_left_out() {
        let bytes = encoded(&[
            lamp(0.0),
            lamp(f32::NAN),
            spot(Vec2::ZERO, DEFAULT_SPOT_ANGLE),
            spot(Vec2::new(1.0, 0.0), MAX_SPOT_ANGLE),
            spot(Vec2::new(2.0, 0.0), DEFAULT_SPOT_ANGLE),
        ]);

        assert_eq!(count(&bytes), 1);
        // The spot's axis is normalized, after its place and radiance.
        let axis_x = 32 + 4 * 8;
        assert_eq!(&bytes[axis_x..axis_x + 4], &1.0f32.to_le_bytes());
    }

    #[test]
    fn sharp_spots_start_fading_at_their_edge() {
        let Light2d::Spot(sharp) = spot(Vec2::new(0.0, 1.0), DEFAULT_SPOT_ANGLE) else {
            panic!("a spot");
        };

        let [_, _, outer, inner] = spot_cone(&sharp).unwrap();

        assert!((outer - inner).abs() < f32::EPSILON);
    }
}
