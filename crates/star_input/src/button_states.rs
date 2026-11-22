//! Per-frame state of a fixed set of buttons (keys, mouse buttons...).

/// Which of `COUNT` buttons are held, and which changed during the current frame.
///
/// Fixed-size arrays: updating it never allocates memory in the game loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ButtonStates<const COUNT: usize> {
    down: [bool; COUNT],
    pressed_this_frame: [bool; COUNT],
    released_this_frame: [bool; COUNT],
}

impl<const COUNT: usize> ButtonStates<COUNT> {
    /// All buttons up, with no changes.
    pub(crate) const fn new() -> Self {
        Self {
            down: [false; COUNT],
            pressed_this_frame: [false; COUNT],
            released_this_frame: [false; COUNT],
        }
    }

    /// Pressing a button that is already held changes nothing.
    pub(crate) fn press(&mut self, index: usize) {
        if !self.down[index] {
            self.down[index] = true;
            self.pressed_this_frame[index] = true;
        }
    }

    /// Releasing a button that is not held changes nothing.
    pub(crate) fn release(&mut self, index: usize) {
        if self.down[index] {
            self.down[index] = false;
            self.released_this_frame[index] = true;
        }
    }

    /// Releases every held button, e.g. when the window loses focus.
    pub(crate) fn release_all(&mut self) {
        for index in 0..COUNT {
            self.release(index);
        }
    }

    /// Forgets this frame's changes; held buttons stay held.
    pub(crate) fn end_frame(&mut self) {
        self.pressed_this_frame = [false; COUNT];
        self.released_this_frame = [false; COUNT];
    }

    pub(crate) const fn is_down(&self, index: usize) -> bool {
        self.down[index]
    }

    pub(crate) const fn just_pressed(&self, index: usize) -> bool {
        self.pressed_this_frame[index]
    }

    pub(crate) const fn just_released(&self, index: usize) -> bool {
        self.released_this_frame[index]
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
        let mut states = buttons();

        states.press(BUTTON);

        assert!(states.is_down(BUTTON));
        assert!(states.just_pressed(BUTTON));
        assert!(!states.just_released(BUTTON));
        assert!(!states.is_down(0));
    }

    #[test]
    fn a_held_button_is_only_just_pressed_in_its_first_frame() {
        let mut states = buttons();
        states.press(BUTTON);

        states.end_frame();

        assert!(states.is_down(BUTTON));
        assert!(!states.just_pressed(BUTTON));
    }

    #[test]
    fn pressing_a_held_button_again_is_not_a_new_press() {
        let mut states = buttons();
        states.press(BUTTON);
        states.end_frame();

        states.press(BUTTON);

        assert!(!states.just_pressed(BUTTON));
    }

    #[test]
    fn a_released_button_is_up_and_just_released() {
        let mut states = buttons();
        states.press(BUTTON);
        states.end_frame();

        states.release(BUTTON);

        assert!(!states.is_down(BUTTON));
        assert!(states.just_released(BUTTON));
    }

    #[test]
    fn releasing_a_button_that_is_up_changes_nothing() {
        let mut states = buttons();

        states.release(BUTTON);

        assert!(!states.just_released(BUTTON));
    }

    #[test]
    fn a_tap_within_one_frame_is_not_lost() {
        let mut states = buttons();

        states.press(BUTTON);
        states.release(BUTTON);

        assert!(!states.is_down(BUTTON));
        assert!(states.just_pressed(BUTTON));
        assert!(states.just_released(BUTTON));
    }

    #[test]
    fn release_all_releases_every_held_button() {
        let mut states = buttons();
        states.press(0);
        states.press(2);

        states.release_all();

        assert!(!states.is_down(0) && !states.is_down(2));
        assert!(states.just_released(0) && states.just_released(2));
        assert!(!states.just_released(BUTTON));
    }
}
