//! Nodes that are bodies of the 2D physics.

use star_math::Vec2;
use star_physics::{BodyId, BodySettings};

use crate::Transform2D;

/// Makes a node a body of the 2D physics: it falls, collides and is
/// pushed (or holds others up), as its [`BodySettings`] say.
///
/// The node needs a [`Transform2D`]. The engine adds the body where the
/// node is the first frame it sees it, and from then on writes where the
/// simulation moved it into the node's [`Transform2D`]; a game that changes
/// that transform itself moves the body there at once. The body's shape is
/// not scaled with the node. Removing the component or the node removes
/// the body. A clone is a new body, added again where its node is, at
/// rest.
///
/// Its velocity can be set and impulses given through it even before the
/// body exists: the engine passes them on before the next step.
#[derive(Debug, PartialEq)]
pub struct Body2D {
    settings: BodySettings,
    /// The body in the physics, once the engine added it.
    body: Option<BodyId>,
    /// The node's transform as the engine last left it, to notice when the
    /// game moves the node.
    written: Option<Transform2D>,
    /// The body's velocity after the last step.
    velocity: Vec2,
    /// The velocity the game gave it, still to pass on.
    new_velocity: Option<Vec2>,
    /// The impulses the game gave it, added up, still to pass on.
    impulse: Vec2,
}

impl Clone for Body2D {
    fn clone(&self) -> Self {
        Self::new(self.settings)
    }
}

impl Body2D {
    /// A body as `settings` say, added to the physics on the next frame.
    #[must_use]
    pub const fn new(settings: BodySettings) -> Self {
        Self {
            settings,
            body: None,
            written: None,
            velocity: Vec2::ZERO,
            new_velocity: None,
            impulse: Vec2::ZERO,
        }
    }

    /// What the body is.
    #[must_use]
    pub const fn settings(&self) -> &BodySettings {
        &self.settings
    }

    /// The body in the physics, to change its velocity or push it. `None`
    /// until the engine adds it, on the frame after the component is added.
    #[must_use]
    pub const fn id(&self) -> Option<BodyId> {
        self.body
    }

    /// How fast the body moves, in world units per second: the velocity
    /// the game set, if the engine did not pass it on yet, or the one it
    /// had after the last step.
    #[must_use]
    pub fn velocity(&self) -> Vec2 {
        self.new_velocity.unwrap_or(self.velocity)
    }

    /// Makes the body move at `velocity`, in world units per second, from
    /// the next step on. Kinematic bodies move only this way.
    pub const fn set_velocity(&mut self, velocity: Vec2) {
        self.new_velocity = Some(velocity);
    }

    /// Pushes the body by `impulse` (mass times velocity) before the next
    /// step, like a jump or a hit. Only dynamic bodies are pushed.
    pub fn apply_impulse(&mut self, impulse: Vec2) {
        self.impulse = self.impulse + impulse;
    }

    /// The velocity and impulse the game gave the body since the engine
    /// last passed them on, leaving none.
    pub(crate) fn take_motion(&mut self) -> (Option<Vec2>, Vec2) {
        (
            self.new_velocity.take(),
            std::mem::replace(&mut self.impulse, Vec2::ZERO),
        )
    }

    pub(crate) const fn measured(&mut self, velocity: Vec2) {
        self.velocity = velocity;
    }

    pub(crate) const fn placed(&mut self, body: BodyId, transform: Transform2D) {
        self.body = Some(body);
        self.written = Some(transform);
    }

    pub(crate) const fn wrote(&mut self, transform: Transform2D) {
        self.written = Some(transform);
    }

    /// Whether the node's `transform` is not the one the engine left.
    pub(crate) fn was_moved(&self, transform: &Transform2D) -> bool {
        self.written.as_ref() != Some(transform)
    }
}
