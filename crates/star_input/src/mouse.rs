//! Mouse state.

use star_math::Vec2;
use star_platform::{ButtonState, MouseButton};

use crate::button_states::ButtonStates;

/// Mouse buttons, cursor position and wheel movement of the current frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Mouse {
    buttons: ButtonStates<{ MouseButton::COUNT }>,
    position: Option<Vec2>,
    wheel: Vec2,
}

impl Mouse {
    /// A mouse with every button up, outside the window and without scrolling.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buttons: ButtonStates::new(),
            position: None,
            wheel: Vec2::ZERO,
        }
    }

    /// Whether `button` is held.
    #[must_use]
    pub const fn is_down(&self, button: MouseButton) -> bool {
        self.buttons.is_down(button.index())
    }

    /// Whether `button` went down during the current frame.
    #[must_use]
    pub const fn just_pressed(&self, button: MouseButton) -> bool {
        self.buttons.just_pressed(button.index())
    }

    /// Whether `button` went up during the current frame.
    #[must_use]
    pub const fn just_released(&self, button: MouseButton) -> bool {
        self.buttons.just_released(button.index())
    }

    /// Cursor position in physical pixels from the window's top-left corner
    /// (Y grows downwards), or `None` while the cursor is outside the window.
    #[must_use]
    pub const fn position(&self) -> Option<Vec2> {
        self.position
    }

    /// Lines scrolled during the current frame: positive X to the right,
    /// positive Y when the wheel turns away from the user.
    #[must_use]
    pub const fn wheel(&self) -> Vec2 {
        self.wheel
    }

    pub(crate) fn set_button(&mut self, button: MouseButton, state: ButtonState) {
        match state {
            ButtonState::Pressed => self.buttons.press(button.index()),
            ButtonState::Released => self.buttons.release(button.index()),
        }
    }

    pub(crate) fn move_cursor(&mut self, position: Vec2) {
        self.position = Some(position);
    }

    pub(crate) fn leave_window(&mut self) {
        self.position = None;
    }

    /// Several wheel events can arrive in one frame; they add up.
    pub(crate) fn scroll(&mut self, lines: Vec2) {
        self.wheel = self.wheel + lines;
    }

    pub(crate) fn release_all(&mut self) {
        self.buttons.release_all();
    }

    pub(crate) fn end_frame(&mut self) {
        self.buttons.end_frame();
        self.wheel = Vec2::ZERO;
    }
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_starts_outside_the_window() {
        assert_eq!(Mouse::new().position(), None);
    }

    #[test]
    fn the_cursor_position_is_kept_until_it_leaves() {
        let mut mouse = Mouse::new();
        mouse.move_cursor(Vec2::new(10.0, 20.0));
        mouse.end_frame();

        assert_eq!(mouse.position(), Some(Vec2::new(10.0, 20.0)));

        mouse.leave_window();

        assert_eq!(mouse.position(), None);
    }

    #[test]
    fn wheel_movement_adds_up_within_a_frame() {
        let mut mouse = Mouse::new();

        mouse.scroll(Vec2::new(0.0, 1.0));
        mouse.scroll(Vec2::new(0.5, 2.0));

        assert_eq!(mouse.wheel(), Vec2::new(0.5, 3.0));
    }

    #[test]
    fn wheel_movement_only_lasts_one_frame() {
        let mut mouse = Mouse::new();
        mouse.scroll(Vec2::new(0.0, 1.0));

        mouse.end_frame();

        assert_eq!(mouse.wheel(), Vec2::ZERO);
    }

    #[test]
    fn buttons_are_tracked() {
        let mut mouse = Mouse::new();

        mouse.set_button(MouseButton::Right, ButtonState::Pressed);

        assert!(mouse.is_down(MouseButton::Right));
        assert!(mouse.just_pressed(MouseButton::Right));
        assert!(!mouse.is_down(MouseButton::Left));
    }
}
