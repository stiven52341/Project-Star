//! What a body is: how it moves, its outline and its surface.

use star_core::{EngineError, EngineResult};

use crate::Shape2d;

/// How a body moves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BodyKind {
    /// Moved by the simulation: gravity, collisions, impulses.
    #[default]
    Dynamic,
    /// Never moves: floors, walls.
    Static,
    /// Moved only by its velocity, which the game sets; it pushes dynamic
    /// bodies but nothing pushes it (moving platforms, doors).
    Kinematic,
}

/// Friction of a surface when none is given: things slide a little.
pub const DEFAULT_FRICTION: f32 = 0.5;
/// Mass per square world unit when none is given.
pub const DEFAULT_DENSITY: f32 = 1.0;

/// What a body is: how it moves, its outline and its surface. Where it is
/// is given when it is added to a [`Physics2d`](crate::Physics2d).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySettings {
    /// How it moves.
    pub kind: BodyKind,
    /// Its outline.
    pub shape: Shape2d,
    /// How much gravity pulls it: 0 floats, 1 falls normally.
    pub gravity_scale: f32,
    /// Whether collisions never turn it (characters that stay upright).
    pub fixed_rotation: bool,
    /// How hard it is to slide over: 0 is ice.
    pub friction: f32,
    /// How much it bounces: 0 stops, 1 bounces back as fast as it came.
    pub bounciness: f32,
    /// Mass per square world unit (only dynamic bodies have mass).
    pub density: f32,
    /// Whether it only detects what overlaps it, without colliding:
    /// areas that open doors, pick up coins or hurt.
    pub is_sensor: bool,
}

impl BodySettings {
    /// A body of `kind` and `shape`, pulled by gravity, with the default
    /// surface.
    #[must_use]
    pub const fn new(kind: BodyKind, shape: Shape2d) -> Self {
        Self {
            kind,
            shape,
            gravity_scale: 1.0,
            fixed_rotation: false,
            friction: DEFAULT_FRICTION,
            bounciness: 0.0,
            density: DEFAULT_DENSITY,
            is_sensor: false,
        }
    }

    /// Checks that a [`Physics2d`](crate::Physics2d) can simulate the body.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if the gravity scale is not a finite
    /// number, the friction or bounciness are negative or not finite, or
    /// the density is not positive.
    pub fn check(&self) -> EngineResult<()> {
        if !self.gravity_scale.is_finite() {
            return Err(EngineError::Physics(format!(
                "the gravity scale of a body must be a finite number, not {}",
                self.gravity_scale
            )));
        }
        let not_negative = [("friction", self.friction), ("bounciness", self.bounciness)];
        if let Some((what, value)) = not_negative
            .iter()
            .find(|(_, value)| !(value.is_finite() && *value >= 0.0))
        {
            return Err(EngineError::Physics(format!(
                "the {what} of a body cannot be negative: {value}"
            )));
        }
        if !(self.density.is_finite() && self.density > 0.0) {
            return Err(EngineError::Physics(format!(
                "the density of a body must be positive, not {}",
                self.density
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> BodySettings {
        BodySettings::new(BodyKind::Dynamic, Shape2d::circle(1.0).unwrap())
    }

    #[test]
    fn default_settings_can_be_simulated() {
        assert!(settings().check().is_ok());
    }

    #[test]
    fn impossible_surfaces_are_physics_errors() {
        let broken: [fn(&mut BodySettings); 4] = [
            |body| body.gravity_scale = f32::NAN,
            |body| body.friction = -1.0,
            |body| body.bounciness = f32::INFINITY,
            |body| body.density = 0.0,
        ];
        for change in broken {
            let mut body = settings();
            change(&mut body);

            assert!(matches!(body.check(), Err(EngineError::Physics(_))));
        }
    }
}
