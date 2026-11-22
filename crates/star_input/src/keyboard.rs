//! Keyboard state.

use star_platform::{ButtonState, Key};

use crate::button_states::ButtonStates;

/// Which keys are held, and which changed during the current frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyboard {
    keys: ButtonStates<{ Key::COUNT }>,
}

impl Keyboard {
    /// A keyboard with every key up.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keys: ButtonStates::new(),
        }
    }

    /// Whether `key` is held.
    #[must_use]
    pub const fn is_down(&self, key: Key) -> bool {
        self.keys.is_down(key.index())
    }

    /// Whether `key` went down during the current frame.
    #[must_use]
    pub const fn just_pressed(&self, key: Key) -> bool {
        self.keys.just_pressed(key.index())
    }

    /// Whether `key` went up during the current frame.
    #[must_use]
    pub const fn just_released(&self, key: Key) -> bool {
        self.keys.just_released(key.index())
    }

    pub(crate) fn set(&mut self, key: Key, state: ButtonState) {
        match state {
            ButtonState::Pressed => self.keys.press(key.index()),
            ButtonState::Released => self.keys.release(key.index()),
        }
    }

    pub(crate) fn release_all(&mut self) {
        self.keys.release_all();
    }

    pub(crate) fn end_frame(&mut self) {
        self.keys.end_frame();
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_tracked_independently() {
        let mut keyboard = Keyboard::new();

        keyboard.set(Key::W, ButtonState::Pressed);

        assert!(keyboard.is_down(Key::W));
        assert!(keyboard.just_pressed(Key::W));
        assert!(!keyboard.is_down(Key::S));
    }

    #[test]
    fn released_keys_report_their_release() {
        let mut keyboard = Keyboard::new();
        keyboard.set(Key::Space, ButtonState::Pressed);
        keyboard.end_frame();

        keyboard.set(Key::Space, ButtonState::Released);

        assert!(!keyboard.is_down(Key::Space));
        assert!(keyboard.just_released(Key::Space));
    }
}
