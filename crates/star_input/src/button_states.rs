//! Per-frame state of buttons (keys, mouse buttons, actions...).

/// Whether one button is held, and whether it changed during the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TrackedButton {
    down: bool,
    pressed_this_frame: bool,
    released_this_frame: bool,
}

impl TrackedButton {
    /// An unheld button, with no changes.
    pub(crate) const UP: Self = Self {
        down: false,
        pressed_this_frame: false,
        released_this_frame: false,
    };

    /// Pressing a button that is already held changes nothing.
    /// Returns whether the button went down.
    pub(crate) fn press(&mut self) -> bool {
        if self.down {
            return false;
        }
        self.down = true;
        self.pressed_this_frame = true;
        true
    }

    /// Releasing a button that is not held changes nothing.
    /// Returns whether the button went up.
    pub(crate) fn release(&mut self) -> bool {
        if !self.down {
            return false;
        }
        self.down = false;
        self.released_this_frame = true;
        true
    }

    /// Forgets this frame's changes; a held button stays held.
    pub(crate) fn end_frame(&mut self) {
        self.pressed_this_frame = false;
        self.released_this_frame = false;
    }

    pub(crate) const fn is_down(self) -> bool {
        self.down
    }

    pub(crate) const fn just_pressed(self) -> bool {
        self.pressed_this_frame
    }

    pub(crate) const fn just_released(self) -> bool {
        self.released_this_frame
    }
}

/// A fixed set of `COUNT` buttons, identified by their index.
///
/// A fixed-size array: updating it never allocates memory in the game loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ButtonStates<const COUNT: usize> {
    buttons: [TrackedButton; COUNT],
}

impl<const COUNT: usize> ButtonStates<COUNT> {
    /// All buttons up, with no changes.
    pub(crate) const fn new() -> Self {
        Self {
            buttons: [TrackedButton::UP; COUNT],
        }
    }

    /// Returns whether the button went down.
    pub(crate) fn press(&mut self, index: usize) -> bool {
        self.buttons[index].press()
    }

    /// Returns whether the button went up.
    pub(crate) fn release(&mut self, index: usize) -> bool {
        self.buttons[index].release()
    }

    /// Releases every held button, e.g. when the window loses focus.
    pub(crate) fn release_all(&mut self) {
        for button in &mut self.buttons {
            button.release();
        }
    }

    /// Forgets this frame's changes; held buttons stay held.
    pub(crate) fn end_frame(&mut self) {
        for button in &mut self.buttons {
            button.end_frame();
        }
    }

    pub(crate) const fn get(&self, index: usize) -> TrackedButton {
        self.buttons[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON: usize = 1;

    fn buttons() -> ButtonStates<3> {
        ButtonStates::new()
    }

    #[test]
    fn a_pressed_button_is_down_and_just_pressed() {
        let mut button = TrackedButton::UP;

        assert!(button.press());

        assert!(button.is_down());
        assert!(button.just_pressed());
        assert!(!button.just_released());
    }

    #[test]
    fn a_held_button_is_only_just_pressed_in_its_first_frame() {
        let mut button = TrackedButton::UP;
        button.press();

        button.end_frame();

        assert!(button.is_down());
        assert!(!button.just_pressed());
    }

    #[test]
    fn pressing_a_held_button_again_is_not_a_new_press() {
        let mut button = TrackedButton::UP;
        button.press();
        button.end_frame();

        assert!(!button.press());
        assert!(!button.just_pressed());
    }

    #[test]
    fn a_released_button_is_up_and_just_released() {
        let mut button = TrackedButton::UP;
        button.press();
        button.end_frame();

        assert!(button.release());

        assert!(!button.is_down());
        assert!(button.just_released());
    }

    #[test]
    fn releasing_a_button_that_is_up_changes_nothing() {
        let mut button = TrackedButton::UP;

        assert!(!button.release());
        assert!(!button.just_released());
    }

    #[test]
    fn a_tap_within_one_frame_is_not_lost() {
        let mut button = TrackedButton::UP;

        button.press();
        button.release();

        assert!(!button.is_down());
        assert!(button.just_pressed());
        assert!(button.just_released());
    }

    #[test]
    fn buttons_of_a_set_are_independent() {
        let mut states = buttons();

        states.press(BUTTON);

        assert!(states.get(BUTTON).is_down());
        assert!(!states.get(0).is_down());
    }

    #[test]
    fn release_all_releases_every_held_button() {
        let mut states = buttons();
        states.press(0);
        states.press(2);

        states.release_all();

        assert!(!states.get(0).is_down() && !states.get(2).is_down());
        assert!(states.get(0).just_released() && states.get(2).just_released());
        assert!(!states.get(BUTTON).just_released());
    }

    #[test]
    fn end_frame_forgets_the_changes_of_every_button() {
        let mut states = buttons();
        states.press(0);

        states.end_frame();

        assert!(states.get(0).is_down());
        assert!(!states.get(0).just_pressed());
    }
}
