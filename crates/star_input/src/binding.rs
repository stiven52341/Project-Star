//! Physical inputs that can trigger an action.

use std::fmt;
use std::str::FromStr;

use star_core::EngineError;
use star_platform::{GamepadButton, Key, MouseButton};

use crate::stick_direction::StickDirection;

/// Prefix of the names of mouse button bindings: `"MouseLeft"`.
const MOUSE_PREFIX: &str = "Mouse";
/// Prefix of the names of gamepad bindings: `"GamepadSouth"`,
/// `"GamepadLeftStickLeft"`.
const GAMEPAD_PREFIX: &str = "Gamepad";
/// Name of the touch binding.
const TOUCH_NAME: &str = "Touch";

/// A physical input bound to an action: while it is held, the action is held.
///
/// Bindings to a device do not care which one it is: a gamepad button
/// binding is held while that button is held on any gamepad.
///
/// Each binding has a name, used in settings files such as `project.toml`
/// (see [`Binding::from_str`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Binding {
    /// A keyboard key.
    Key(Key),
    /// A mouse button.
    MouseButton(MouseButton),
    /// Any finger on the touch screen, e.g. "tap anywhere to jump".
    Touch,
    /// A button of any gamepad. With [`crate::Input::action_strength`],
    /// the triggers give how far they are pressed.
    GamepadButton(GamepadButton),
    /// A stick of any gamepad leaning one way: held from
    /// [`crate::STICK_PRESS_THRESHOLD`], and analog with
    /// [`crate::Input::action_strength`].
    GamepadStick(StickDirection),
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

impl From<GamepadButton> for Binding {
    fn from(button: GamepadButton) -> Self {
        Self::GamepadButton(button)
    }
}

impl From<StickDirection> for Binding {
    fn from(direction: StickDirection) -> Self {
        Self::GamepadStick(direction)
    }
}

impl FromStr for Binding {
    type Err = EngineError;

    /// Reads a binding name: a key (`"Space"`, `"A"`, `"ArrowLeft"`), a mouse
    /// button (`"MouseLeft"`), `"Touch"`, a gamepad button (`"GamepadSouth"`)
    /// or a stick direction (`"GamepadLeftStickLeft"`).
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let binding = if name == TOUCH_NAME {
            Some(Self::Touch)
        } else if let Some(button) = name.strip_prefix(MOUSE_PREFIX) {
            MouseButton::from_name(button).map(Self::MouseButton)
        } else if let Some(input) = name.strip_prefix(GAMEPAD_PREFIX) {
            GamepadButton::from_name(input)
                .map(Self::GamepadButton)
                .or_else(|| StickDirection::from_name(input).map(Self::GamepadStick))
        } else {
            Key::from_name(name).map(Self::Key)
        };
        binding.ok_or_else(|| EngineError::Input(format!("unknown input binding \"{name}\"")))
    }
}

impl fmt::Display for Binding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(key) => formatter.write_str(key.name()),
            Self::MouseButton(button) => write!(formatter, "{MOUSE_PREFIX}{}", button.name()),
            Self::Touch => formatter.write_str(TOUCH_NAME),
            Self::GamepadButton(button) => write!(formatter, "{GAMEPAD_PREFIX}{}", button.name()),
            Self::GamepadStick(direction) => {
                write!(formatter, "{GAMEPAD_PREFIX}{}", direction.name())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_binding_is_read_by_name() {
        assert_eq!("Space".parse(), Ok(Binding::Key(Key::Space)));
        assert_eq!(
            "MouseLeft".parse(),
            Ok(Binding::MouseButton(MouseButton::Left))
        );
        assert_eq!("Touch".parse(), Ok(Binding::Touch));
        assert_eq!(
            "GamepadSouth".parse(),
            Ok(Binding::GamepadButton(GamepadButton::South))
        );
        assert_eq!(
            "GamepadLeftStickLeft".parse(),
            Ok(Binding::GamepadStick(StickDirection::LeftStickLeft))
        );
        assert_eq!(
            "GamepadLeftStick".parse(),
            Ok(Binding::GamepadButton(GamepadButton::LeftStick))
        );
    }

    #[test]
    fn names_round_trip() {
        let bindings = [
            Binding::Key(Key::ArrowLeft),
            Binding::MouseButton(MouseButton::Right),
            Binding::Touch,
            Binding::GamepadButton(GamepadButton::DPadUp),
            Binding::GamepadStick(StickDirection::RightStickDown),
        ];
        for binding in bindings {
            assert_eq!(binding.to_string().parse(), Ok(binding));
        }
    }

    #[test]
    fn unknown_names_are_an_input_error() {
        assert!(matches!(
            "Jump".parse::<Binding>(),
            Err(EngineError::Input(_))
        ));
        assert!("MouseSide".parse::<Binding>().is_err());
        assert!("GamepadX".parse::<Binding>().is_err());
    }
}
