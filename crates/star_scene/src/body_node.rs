//! Nodes that are bodies of the 2D physics.

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
/// the body. A clone is a new body, added again where its node is.
#[derive(Debug, PartialEq)]
pub struct Body2D {
    settings: BodySettings,
    /// The body in the physics, once the engine added it.
    body: Option<BodyId>,
    /// The node's transform as the engine last left it, to notice when the
    /// game moves the node.
    written: Option<Transform2D>,
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
