//! Physical inputs that can trigger an action.

use star_platform::{GamepadButton, Key, MouseButton};

/// A physical input bound to an action: while it is held, the action is held.
///
/// Bindings to a device do not care which one it is: a gamepad button
/// binding is held while that button is held on any gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Binding {
    /// A keyboard key.
    Key(Key),
    /// A mouse button.
    MouseButton(MouseButton),
    /// Any finger on the touch screen, e.g. "tap anywhere to jump".
    Touch,
    /// A button of any gamepad.
    GamepadButton(GamepadButton),
}

impl From<Key> for Binding {
    fn from(key: Key) -> Self {
        Self::Key(key)
    }
}

impl From<GamepadButton> for Binding {
    fn from(button: GamepadButton) -> Self {
        Self::GamepadButton(button)
    }
}

impl From<MouseButton> for Binding {
    fn from(button: MouseButton) -> Self {
        Self::MouseButton(button)
    }
}
