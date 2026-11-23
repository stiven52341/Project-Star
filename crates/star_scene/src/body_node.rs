//! Nodes that are bodies of the 2D or 3D physics.

use std::ops::Add;

use star_math::{Vec2, Vec3};
use star_physics::{BodyId, BodyId3d, BodySettings, Shape2d, Shape3d};

use crate::{Transform2D, Transform3D};

/// Makes a 2D node a body of the 2D physics (a
/// [`Physics2d`](star_physics::Physics2d)); see [`NodeBody`].
pub type Body2D = NodeBody<Shape2d, BodyId, Transform2D, Vec2>;

/// Makes a 3D node a body of the 3D physics (a
/// [`Physics3d`](star_physics::Physics3d)); see [`NodeBody`].
pub type Body3D = NodeBody<Shape3d, BodyId3d, Transform3D, Vec3>;

/// Makes a node a body of the physics: it falls, collides and is pushed
/// (or holds others up), as its [`BodySettings`] say. Used as [`Body2D`]
/// on 2D nodes and [`Body3D`] on 3D nodes.
///
/// The node needs a transform of its kind ([`Transform2D`] or
/// [`Transform3D`]). The engine adds the body where the node is the first
/// frame it sees it, and from then on writes where the simulation moved
/// it into the node's transform; a game that changes that transform
/// itself moves the body there at once. The body's shape is not scaled
/// with the node. Removing the component or the node removes the body. A
/// clone is a new body, added again where its node is, at rest.
///
/// Its velocity can be set and impulses given through it even before the
/// body exists: the engine passes them on before the next step.
#[derive(Debug, PartialEq)]
pub struct NodeBody<Shape, Id, Transform, Vector> {
    settings: BodySettings<Shape>,
    /// The body in the physics, once the engine added it.
    body: Option<Id>,
    /// The node's transform as the engine last left it, to notice when the
    /// game moves the node.
    written: Option<Transform>,
    /// The body's velocity after the last step.
    velocity: Vector,
    /// The velocity the game gave it, still to pass on.
    new_velocity: Option<Vector>,
    /// The impulses the game gave it, added up, still to pass on.
    impulse: Vector,
}

impl<Shape, Id, Transform, Vector> Clone for NodeBody<Shape, Id, Transform, Vector>
where
    Shape: Copy,
    Id: Copy,
    Transform: Copy + PartialEq,
    Vector: Copy + Default + PartialEq + Add<Output = Vector>,
{
    fn clone(&self) -> Self {
        Self::new(self.settings)
    }
}

impl<Shape, Id, Transform, Vector> NodeBody<Shape, Id, Transform, Vector>
where
    Id: Copy,
    Transform: Copy + PartialEq,
    Vector: Copy + Default + PartialEq + Add<Output = Vector>,
{
    /// A body as `settings` say, added to the physics on the next frame.
    #[must_use]
    pub fn new(settings: BodySettings<Shape>) -> Self {
        Self {
            settings,
            body: None,
            written: None,
            velocity: Vector::default(),
            new_velocity: None,
            impulse: Vector::default(),
        }
    }

    /// What the body is.
    #[must_use]
    pub const fn settings(&self) -> &BodySettings<Shape> {
        &self.settings
    }

    /// The body in the physics, to change its velocity or push it. `None`
    /// until the engine adds it, on the frame after the component is added.
    #[must_use]
    pub const fn id(&self) -> Option<Id> {
        self.body
    }

    /// How fast the body moves, in world units per second: the velocity
    /// the game set, if the engine did not pass it on yet, or the one it
    /// had after the last step.
    #[must_use]
    pub fn velocity(&self) -> Vector {
        self.new_velocity.unwrap_or(self.velocity)
    }

    /// Makes the body move at `velocity`, in world units per second, from
    /// the next step on. Kinematic bodies move only this way.
    pub const fn set_velocity(&mut self, velocity: Vector) {
        self.new_velocity = Some(velocity);
    }

    /// Pushes the body by `impulse` (mass times velocity) before the next
    /// step, like a jump or a hit. Only dynamic bodies are pushed.
    pub fn apply_impulse(&mut self, impulse: Vector) {
        self.impulse = self.impulse + impulse;
    }

    /// The velocity and impulse the game gave the body since the engine
    /// last passed them on, leaving none; `None` for an impulse of zero.
    pub(crate) fn take_motion(&mut self) -> (Option<Vector>, Option<Vector>) {
        let impulse = std::mem::take(&mut self.impulse);
        (
            self.new_velocity.take(),
            (impulse != Vector::default()).then_some(impulse),
        )
    }

    pub(crate) const fn measured(&mut self, velocity: Vector) {
        self.velocity = velocity;
    }

    pub(crate) const fn placed(&mut self, body: Id, transform: Transform) {
        self.body = Some(body);
        self.written = Some(transform);
    }

    pub(crate) const fn wrote(&mut self, transform: Transform) {
        self.written = Some(transform);
    }

    /// Whether the node's `transform` is not the one the engine left.
    pub(crate) fn was_moved(&self, transform: &Transform) -> bool {
        self.written.as_ref() != Some(transform)
    }
}
