//! How a body moves and what its surface is like, when it is added.

use star_math::Vec2;

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

/// Everything about a body when it is added to a
/// [`Physics2d`](crate::Physics2d).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySettings {
    /// How it moves.
    pub kind: BodyKind,
    /// Its outline.
    pub shape: Shape2d,
    /// Where its center starts, in world units.
    pub position: Vec2,
    /// Its starting angle, in radians, counterclockwise.
    pub rotation: f32,
    /// Its starting velocity, in world units per second.
    pub velocity: Vec2,
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
    /// A body of `kind` and `shape` at the origin, at rest, with the
    /// default surface.
    #[must_use]
    pub const fn new(kind: BodyKind, shape: Shape2d) -> Self {
        Self {
            kind,
            shape,
            position: Vec2::ZERO,
            rotation: 0.0,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            fixed_rotation: false,
            friction: DEFAULT_FRICTION,
            bounciness: 0.0,
            density: DEFAULT_DENSITY,
            is_sensor: false,
        }
    }

    /// The same body, starting at `position`.
    #[must_use]
    pub const fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }
}
