//! The physics bodies of StarScript nodes, as the generated game describes
//! them to [`ScriptNodes`](crate::script_nodes::ScriptNodes).
//!
//! StarScript has no enums yet, so a script's `Body` names how it moves and
//! its shape with words, and keeps the sizes of every shape; only those of
//! its shape are used.

use star_core::{EngineError, EngineResult};
use star_math::Vec2;
use star_physics::{BodyKind, BodySettings, Shape2d, ShapeKind};
use star_scene::Transform2D;

/// How StarScript names [`BodyKind::Dynamic`].
pub const DYNAMIC: &str = "dynamic";
/// How StarScript names [`BodyKind::Static`].
pub const STATIC: &str = "static";
/// How StarScript names [`BodyKind::Kinematic`].
pub const KINEMATIC: &str = "kinematic";
/// How StarScript names circles.
pub const CIRCLE: &str = "circle";
/// How StarScript names rectangles.
pub const RECTANGLE: &str = "rectangle";
/// How StarScript names capsules.
pub const CAPSULE: &str = "capsule";

/// The radius of a script body whose shape does not give one.
pub const DEFAULT_RADIUS: f32 = 0.5;
/// The width and height of a script body whose shape does not give them.
pub const DEFAULT_SIZE: f32 = 1.0;

/// A StarScript `Body`: what the body is, and what the script asked it to
/// do during its frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptBody<'a> {
    /// [`DYNAMIC`], [`STATIC`] or [`KINEMATIC`].
    pub kind: &'a str,
    /// [`CIRCLE`], [`RECTANGLE`] or [`CAPSULE`].
    pub shape: &'a str,
    /// Of circles and capsules.
    pub radius: f32,
    /// Of rectangles.
    pub width: f32,
    /// Of rectangles and capsules (half circles included).
    pub height: f32,
    /// How much gravity pulls it.
    pub gravity_scale: f32,
    /// Whether collisions never turn it.
    pub fixed_rotation: bool,
    /// How hard it is to slide over.
    pub friction: f32,
    /// How much it bounces.
    pub bounciness: f32,
    /// Mass per square world unit.
    pub density: f32,
    /// Whether it only detects what overlaps it.
    pub sensor: bool,
    /// The velocity the script's `Body` has, in world units per second.
    pub velocity: Vec2,
    /// The impulses the script gave it during its frame, added up.
    pub impulse: Vec2,
}

/// What the engine tells a StarScript `Body` before the script's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptBodyState<'a> {
    /// Where the simulation left its node, relative to the node's parent.
    pub transform: Transform2D,
    /// How fast it moves, in world units per second.
    pub velocity: Vec2,
    /// The script nodes whose bodies touch it (or overlap it, for sensors).
    pub touching: &'a [u64],
    /// The script nodes that started touching it during the last frame.
    pub just_touched: &'a [u64],
    /// The script nodes that stopped touching it during the last frame.
    pub just_separated: &'a [u64],
}

impl ScriptBody<'static> {
    /// The script body that `settings` describe, at rest: how scenes give
    /// bodies to script nodes.
    #[must_use]
    pub fn from_settings(settings: &BodySettings) -> Self {
        let kind = match settings.kind {
            BodyKind::Dynamic => DYNAMIC,
            BodyKind::Static => STATIC,
            BodyKind::Kinematic => KINEMATIC,
        };
        let (shape, radius, width, height) = match settings.shape.kind() {
            ShapeKind::Circle { radius } => (CIRCLE, radius, DEFAULT_SIZE, DEFAULT_SIZE),
            ShapeKind::Rectangle { size } => (RECTANGLE, DEFAULT_RADIUS, size.x, size.y),
            ShapeKind::Capsule { height, radius } => (CAPSULE, radius, DEFAULT_SIZE, height),
        };
        Self {
            kind,
            shape,
            radius,
            width,
            height,
            gravity_scale: settings.gravity_scale,
            fixed_rotation: settings.fixed_rotation,
            friction: settings.friction,
            bounciness: settings.bounciness,
            density: settings.density,
            sensor: settings.is_sensor,
            velocity: Vec2::ZERO,
            impulse: Vec2::ZERO,
        }
    }
}

impl ScriptBody<'_> {
    /// The body to simulate.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if the kind or shape is not a known word,
    /// or the physics cannot simulate the body (see
    /// [`BodySettings::check`]).
    pub fn to_settings(&self) -> EngineResult<BodySettings> {
        let kind = match self.kind {
            DYNAMIC => BodyKind::Dynamic,
            STATIC => BodyKind::Static,
            KINEMATIC => BodyKind::Kinematic,
            other => {
                return Err(EngineError::Physics(format!(
                    "a body is \"{DYNAMIC}\", \"{STATIC}\" or \"{KINEMATIC}\", not \"{other}\""
                )));
            }
        };
        let shape = match self.shape {
            CIRCLE => Shape2d::circle(self.radius),
            RECTANGLE => Shape2d::rectangle(Vec2::new(self.width, self.height)),
            CAPSULE => Shape2d::capsule(self.height, self.radius),
            other => {
                return Err(EngineError::Physics(format!(
                    "a body is a \"{CIRCLE}\", a \"{RECTANGLE}\" or a \"{CAPSULE}\", not a \"{other}\""
                )));
            }
        }
        .map_err(|error| EngineError::Physics(error.to_string()))?;
        let settings = BodySettings {
            gravity_scale: self.gravity_scale,
            fixed_rotation: self.fixed_rotation,
            friction: self.friction,
            bounciness: self.bounciness,
            density: self.density,
            is_sensor: self.sensor,
            ..BodySettings::new(kind, shape)
        };
        settings.check()?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_become_script_bodies_and_back() {
        let shapes = [
            Shape2d::circle(0.3).unwrap(),
            Shape2d::rectangle(Vec2::new(4.0, 1.0)).unwrap(),
            Shape2d::capsule(2.0, 0.5).unwrap(),
        ];
        for (kind, shape) in [BodyKind::Dynamic, BodyKind::Static, BodyKind::Kinematic]
            .into_iter()
            .zip(shapes)
        {
            let mut settings = BodySettings::new(kind, shape);
            settings.bounciness = 0.7;
            settings.is_sensor = true;

            let body = ScriptBody::from_settings(&settings);

            assert_eq!(body.to_settings().unwrap(), settings);
        }
    }

    #[test]
    fn unknown_words_and_impossible_bodies_are_physics_errors() {
        let body = ScriptBody::from_settings(&BodySettings::new(
            BodyKind::Dynamic,
            Shape2d::circle(1.0).unwrap(),
        ));
        let broken = [
            ScriptBody {
                kind: "flying",
                ..body
            },
            ScriptBody {
                shape: "triangle",
                ..body
            },
            ScriptBody {
                radius: 0.0,
                ..body
            },
            ScriptBody {
                density: -1.0,
                ..body
            },
        ];
        for body in broken {
            assert!(
                matches!(body.to_settings(), Err(EngineError::Physics(_))),
                "{body:?}"
            );
        }
    }
}
