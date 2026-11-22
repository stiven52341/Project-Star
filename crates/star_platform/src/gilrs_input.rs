//! Gamepads through `gilrs`, translated into [`InputEvent`]s.
//!
//! The OS does not deliver gamepad events through the window, so they are
//! polled once per frame.

use gilrs::{Axis, Button, EventType, Gilrs};

use crate::input_event::{ButtonState, GamepadAxis, GamepadButton, InputEvent};

/// Source of gamepad events.
pub(crate) struct GamepadEvents {
    gilrs: Gilrs,
    reported_initial_gamepads: bool,
}

impl GamepadEvents {
    /// Starts listening to gamepads, or `None` if the system does not allow it.
    /// Games still run without gamepads, so the failure is only logged.
    pub(crate) fn new() -> Option<Self> {
        let gilrs = match Gilrs::new() {
            // `NotImplemented` brings a context without gamepads: the platform
            // has no gamepad support yet, but the game can run the same way.
            Ok(gilrs) | Err(gilrs::Error::NotImplemented(gilrs)) => gilrs,
            Err(error) => {
                log::warn!("Gamepads are not available: {error}");
                return None;
            }
        };
        Some(Self {
            gilrs,
            reported_initial_gamepads: false,
        })
    }

    /// Reports every gamepad event since the last call.
    pub(crate) fn poll(&mut self, mut report: impl FnMut(InputEvent)) {
        if !self.reported_initial_gamepads {
            self.reported_initial_gamepads = true;
            for (id, _) in self.gilrs.gamepads() {
                report(InputEvent::GamepadConnected { gamepad: id.into() });
            }
        }
        while let Some(event) = self.gilrs.next_event() {
            if let Some(input) = translate_event(event.id.into(), event.event) {
                report(input);
            }
        }
    }
}

fn translate_event(gamepad: usize, event: EventType) -> Option<InputEvent> {
    match event {
        EventType::Connected => Some(InputEvent::GamepadConnected { gamepad }),
        EventType::Disconnected => Some(InputEvent::GamepadDisconnected { gamepad }),
        EventType::ButtonPressed(button, _) => button_event(gamepad, button, ButtonState::Pressed),
        EventType::ButtonReleased(button, _) => {
            button_event(gamepad, button, ButtonState::Released)
        }
        EventType::ButtonChanged(button, value, _) => trigger_travel(gamepad, button, value),
        EventType::AxisChanged(axis, value, _) => axis_event(gamepad, axis, value),
        // Repeats, dropped events, force feedback and kinds added by later
        // versions of `gilrs` (the enum is non-exhaustive) mean nothing to games.
        _ => None,
    }
}

fn button_event(gamepad: usize, button: Button, state: ButtonState) -> Option<InputEvent> {
    translate_button(button).map(|button| InputEvent::GamepadButton {
        gamepad,
        button,
        state,
    })
}

/// Analog buttons report their travel; only the triggers have a meaningful one.
fn trigger_travel(gamepad: usize, button: Button, value: f32) -> Option<InputEvent> {
    let axis = match button {
        Button::LeftTrigger2 => GamepadAxis::LeftTrigger,
        Button::RightTrigger2 => GamepadAxis::RightTrigger,
        _ => return None,
    };
    Some(InputEvent::GamepadAxis {
        gamepad,
        axis,
        value,
    })
}

fn axis_event(gamepad: usize, axis: Axis, value: f32) -> Option<InputEvent> {
    translate_axis(axis).map(|axis| InputEvent::GamepadAxis {
        gamepad,
        axis,
        value,
    })
}

/// `gilrs` calls the shoulder buttons "triggers" and the triggers "triggers 2".
fn translate_button(button: Button) -> Option<GamepadButton> {
    match button {
        Button::South => Some(GamepadButton::South),
        Button::East => Some(GamepadButton::East),
        Button::North => Some(GamepadButton::North),
        Button::West => Some(GamepadButton::West),
        Button::LeftTrigger => Some(GamepadButton::LeftBumper),
        Button::RightTrigger => Some(GamepadButton::RightBumper),
        Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger),
        Button::RightTrigger2 => Some(GamepadButton::RightTrigger),
        Button::Select => Some(GamepadButton::Select),
        Button::Start => Some(GamepadButton::Start),
        Button::Mode => Some(GamepadButton::Mode),
        Button::LeftThumb => Some(GamepadButton::LeftStick),
        Button::RightThumb => Some(GamepadButton::RightStick),
        Button::DPadUp => Some(GamepadButton::DPadUp),
        Button::DPadDown => Some(GamepadButton::DPadDown),
        Button::DPadLeft => Some(GamepadButton::DPadLeft),
        Button::DPadRight => Some(GamepadButton::DPadRight),
        Button::C | Button::Z | Button::Unknown => None,
    }
}

/// The directional pad is reported as buttons (`gilrs` converts its axes),
/// and the triggers through [`trigger_travel`].
fn translate_axis(axis: Axis) -> Option<GamepadAxis> {
    match axis {
        Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
        Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
        Axis::RightStickX => Some(GamepadAxis::RightStickX),
        Axis::RightStickY => Some(GamepadAxis::RightStickY),
        Axis::LeftZ | Axis::RightZ | Axis::DPadX | Axis::DPadY | Axis::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAMEPAD: usize = 3;

    #[test]
    fn shoulder_buttons_and_triggers_get_clear_names() {
        assert_eq!(
            translate_button(Button::LeftTrigger),
            Some(GamepadButton::LeftBumper)
        );
        assert_eq!(
            translate_button(Button::RightTrigger2),
            Some(GamepadButton::RightTrigger)
        );
    }

    #[test]
    fn unusual_buttons_are_ignored() {
        assert_eq!(translate_button(Button::C), None);
        assert_eq!(translate_button(Button::Unknown), None);
    }

    #[test]
    fn button_events_keep_their_gamepad_and_state() {
        assert_eq!(
            button_event(GAMEPAD, Button::South, ButtonState::Pressed),
            Some(InputEvent::GamepadButton {
                gamepad: GAMEPAD,
                button: GamepadButton::South,
                state: ButtonState::Pressed,
            })
        );
    }

    #[test]
    fn trigger_travel_becomes_a_trigger_axis() {
        assert_eq!(
            trigger_travel(GAMEPAD, Button::LeftTrigger2, 0.5),
            Some(InputEvent::GamepadAxis {
                gamepad: GAMEPAD,
                axis: GamepadAxis::LeftTrigger,
                value: 0.5,
            })
        );
        assert_eq!(trigger_travel(GAMEPAD, Button::South, 0.5), None);
    }

    #[test]
    fn only_stick_axes_are_reported_as_axes() {
        assert_eq!(
            axis_event(GAMEPAD, Axis::RightStickY, -1.0),
            Some(InputEvent::GamepadAxis {
                gamepad: GAMEPAD,
                axis: GamepadAxis::RightStickY,
                value: -1.0,
            })
        );
        assert_eq!(axis_event(GAMEPAD, Axis::DPadX, 1.0), None);
    }

    #[test]
    fn connections_are_reported() {
        assert_eq!(
            translate_event(GAMEPAD, EventType::Disconnected),
            Some(InputEvent::GamepadDisconnected { gamepad: GAMEPAD })
        );
    }
}
