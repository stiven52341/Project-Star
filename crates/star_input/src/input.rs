//! State of every input device, updated from the platform's input events.

use star_math::Vec2;
use star_platform::InputEvent;

use crate::keyboard::Keyboard;
use crate::mouse::Mouse;

/// Everything the player is doing with their input devices this frame.
///
/// The engine feeds it with [`Input::apply`] as events arrive and calls
/// [`Input::end_frame`] after each update; games only read it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Input {
    keyboard: Keyboard,
    mouse: Mouse,
}

impl Input {
    /// Input with no key or button held.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),
        }
    }

    /// State of the keyboard.
    #[must_use]
    pub const fn keyboard(&self) -> &Keyboard {
        &self.keyboard
    }

    /// State of the mouse.
    #[must_use]
    pub const fn mouse(&self) -> &Mouse {
        &self.mouse
    }

    /// Updates the state with something the user did.
    pub fn apply(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, state } => self.keyboard.set(key, state),
            InputEvent::MouseButton { button, state } => self.mouse.set_button(button, state),
            InputEvent::CursorMoved { x, y } => self.mouse.move_cursor(Vec2::new(x, y)),
            InputEvent::CursorLeft => self.mouse.leave_window(),
            InputEvent::MouseWheel { x, y } => self.mouse.scroll(Vec2::new(x, y)),
            InputEvent::FocusLost => self.release_everything(),
        }
    }

    /// Ends the current frame: forgets what was just pressed, released or
    /// scrolled, and keeps what is still held.
    pub fn end_frame(&mut self) {
        self.keyboard.end_frame();
        self.mouse.end_frame();
    }

    fn release_everything(&mut self) {
        self.keyboard.release_all();
        self.mouse.release_all();
    }
}

#[cfg(test)]
mod tests {
    use star_platform::{ButtonState, Key, MouseButton};

    use super::*;

    fn press(key: Key) -> InputEvent {
        InputEvent::Key {
            key,
            state: ButtonState::Pressed,
        }
    }

    #[test]
    fn key_events_reach_the_keyboard() {
        let mut input = Input::new();

        input.apply(press(Key::A));

        assert!(input.keyboard().just_pressed(Key::A));
    }

    #[test]
    fn mouse_events_reach_the_mouse() {
        let mut input = Input::new();

        input.apply(InputEvent::MouseButton {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        });
        input.apply(InputEvent::CursorMoved { x: 3.0, y: 4.0 });
        input.apply(InputEvent::MouseWheel { x: 0.0, y: -1.0 });

        assert!(input.mouse().is_down(MouseButton::Left));
        assert_eq!(input.mouse().position(), Some(Vec2::new(3.0, 4.0)));
        assert_eq!(input.mouse().wheel(), Vec2::new(0.0, -1.0));

        input.apply(InputEvent::CursorLeft);

        assert_eq!(input.mouse().position(), None);
    }

    #[test]
    fn losing_focus_releases_every_key_and_button() {
        let mut input = Input::new();
        input.apply(press(Key::W));
        input.apply(InputEvent::MouseButton {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        });
        input.end_frame();

        input.apply(InputEvent::FocusLost);

        assert!(!input.keyboard().is_down(Key::W));
        assert!(input.keyboard().just_released(Key::W));
        assert!(!input.mouse().is_down(MouseButton::Left));
    }

    #[test]
    fn ending_the_frame_keeps_held_keys() {
        let mut input = Input::new();
        input.apply(press(Key::D));

        input.end_frame();

        assert!(input.keyboard().is_down(Key::D));
        assert!(!input.keyboard().just_pressed(Key::D));
    }
}
