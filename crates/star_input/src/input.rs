//! State of every input device, updated from the platform's input events.

use star_math::Vec2;
use star_platform::{ButtonState, InputEvent, TouchPhase};

use crate::action_map::ActionMap;
use crate::actions::Actions;
use crate::binding::Binding;
use crate::keyboard::Keyboard;
use crate::mouse::Mouse;
use crate::touches::Touches;

/// Everything the player is doing with their input devices this frame.
///
/// Games normally ask for [`Input::actions`], which work with any device;
/// the keyboard and the mouse are there for what is specific to them.
///
/// The engine feeds it with [`Input::apply`] as events arrive and calls
/// [`Input::end_frame`] after each update; games only read it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Input {
    keyboard: Keyboard,
    mouse: Mouse,
    touches: Touches,
    actions: Actions,
}

impl Input {
    /// Input without actions, with no key or button held.
    #[must_use]
    pub fn new() -> Self {
        Self::with_actions(ActionMap::new())
    }

    /// Input that tracks the actions of `map`.
    #[must_use]
    pub fn with_actions(map: ActionMap) -> Self {
        Self {
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),
            touches: Touches::new(),
            actions: Actions::new(map),
        }
    }

    /// State of the game's actions.
    #[must_use]
    pub const fn actions(&self) -> &Actions {
        &self.actions
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

    /// Fingers on the touch screen.
    #[must_use]
    pub const fn touches(&self) -> &Touches {
        &self.touches
    }

    /// Updates the state with something the user did.
    pub fn apply(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, state } => {
                if self.keyboard.set(key, state) {
                    self.actions.binding_changed(Binding::Key(key), state);
                }
            }
            InputEvent::MouseButton { button, state } => {
                if self.mouse.set_button(button, state) {
                    self.actions
                        .binding_changed(Binding::MouseButton(button), state);
                }
            }
            InputEvent::CursorMoved { x, y } => self.mouse.move_cursor(Vec2::new(x, y)),
            InputEvent::CursorLeft => self.mouse.leave_window(),
            InputEvent::MouseWheel { x, y } => self.mouse.scroll(Vec2::new(x, y)),
            InputEvent::Touch { id, phase, x, y } => self.touch(id, phase, Vec2::new(x, y)),
            InputEvent::FocusLost => self.release_everything(),
        }
    }

    /// Ends the current frame: forgets what was just pressed, released or
    /// scrolled, and keeps what is still held.
    pub fn end_frame(&mut self) {
        self.keyboard.end_frame();
        self.mouse.end_frame();
        self.touches.end_frame();
        self.actions.end_frame();
    }

    /// The [`Binding::Touch`] actions follow whether any finger is down.
    fn touch(&mut self, id: u64, phase: TouchPhase, position: Vec2) {
        let was_touching = self.touches.is_any_down();
        self.touches.apply(id, phase, position);
        let is_touching = self.touches.is_any_down();
        if is_touching != was_touching {
            let state = if is_touching {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            };
            self.actions.binding_changed(Binding::Touch, state);
        }
    }

    fn release_everything(&mut self) {
        self.keyboard.release_all();
        self.mouse.release_all();
        self.touches.release_all();
        self.actions.release_all();
    }
}

#[cfg(test)]
mod tests {
    use star_platform::{Key, MouseButton};

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

    fn jump_input() -> Input {
        let mut map = ActionMap::new();
        map.add(JUMP, &[Key::Space.into(), MouseButton::Left.into()])
            .unwrap();
        Input::with_actions(map)
    }

    const JUMP: &str = "jump";

    #[test]
    fn keys_and_buttons_trigger_their_actions() {
        let mut input = jump_input();

        input.apply(InputEvent::MouseButton {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        });

        assert!(input.actions().just_pressed(JUMP));
    }

    #[test]
    fn repeated_press_events_do_not_count_twice_for_actions() {
        let mut input = jump_input();
        input.apply(press(Key::Space));
        input.apply(press(Key::Space));

        input.apply(InputEvent::Key {
            key: Key::Space,
            state: ButtonState::Released,
        });

        assert!(!input.actions().is_down(JUMP));
    }

    #[test]
    fn losing_focus_releases_every_action() {
        let mut input = jump_input();
        input.apply(press(Key::Space));

        input.apply(InputEvent::FocusLost);

        assert!(!input.actions().is_down(JUMP));
    }

    #[test]
    fn ending_the_frame_forgets_action_changes() {
        let mut input = jump_input();
        input.apply(press(Key::Space));

        input.end_frame();

        assert!(input.actions().is_down(JUMP));
        assert!(!input.actions().just_pressed(JUMP));
    }

    fn touch(id: u64, phase: TouchPhase) -> InputEvent {
        InputEvent::Touch {
            id,
            phase,
            x: 1.0,
            y: 2.0,
        }
    }

    fn tap_input() -> Input {
        let mut map = ActionMap::new();
        map.add(JUMP, &[Binding::Touch]).unwrap();
        Input::with_actions(map)
    }

    #[test]
    fn touch_events_reach_the_touches() {
        let mut input = Input::new();

        input.apply(touch(1, TouchPhase::Started));

        assert!(input.touches().is_any_down());
        assert_eq!(input.touches().iter().count(), 1);
    }

    #[test]
    fn touching_the_screen_triggers_touch_actions() {
        let mut input = tap_input();

        input.apply(touch(1, TouchPhase::Started));

        assert!(input.actions().just_pressed(JUMP));
    }

    #[test]
    fn touch_actions_stay_held_until_the_last_finger_leaves() {
        let mut input = tap_input();
        input.apply(touch(1, TouchPhase::Started));
        input.apply(touch(2, TouchPhase::Started));
        input.end_frame();

        input.apply(touch(1, TouchPhase::Ended));

        assert!(input.actions().is_down(JUMP));

        input.apply(touch(2, TouchPhase::Ended));

        assert!(input.actions().just_released(JUMP));
    }

    #[test]
    fn losing_focus_lifts_every_finger() {
        let mut input = tap_input();
        input.apply(touch(1, TouchPhase::Started));

        input.apply(InputEvent::FocusLost);

        assert!(!input.touches().is_any_down());
        assert!(!input.actions().is_down(JUMP));
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
