//! Physical inputs that can trigger an action.

use star_platform::{Key, MouseButton};

/// A physical input bound to an action: while it is held, the action is held.
///
/// Gamepads will be added as a new kind of binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Binding {
    /// A keyboard key.
    Key(Key),
    /// A mouse button.
    MouseButton(MouseButton),
    /// Any finger on the touch screen, e.g. "tap anywhere to jump".
    Touch,
}

impl From<Key> for Binding {
    fn from(key: Key) -> Self {
        Self::Key(key)
    }
}

impl From<MouseButton> for Binding {
    fn from(button: MouseButton) -> Self {
        Self::MouseButton(button)
    }
}
