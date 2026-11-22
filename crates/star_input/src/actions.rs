//! Per-frame state of the game's actions.

use star_platform::ButtonState;

use crate::action_map::{ActionBindings, ActionMap};
use crate::binding::Binding;
use crate::button_states::TrackedButton;

/// One action while the game runs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionState {
    action: ActionBindings,
    /// How many of its bindings are held: the action is held while it is not zero.
    held_bindings: usize,
    button: TrackedButton,
}

impl ActionState {
    fn binding_pressed(&mut self) {
        self.held_bindings += 1;
        self.button.press();
    }

    fn binding_released(&mut self) {
        self.held_bindings = self.held_bindings.saturating_sub(1);
        if self.held_bindings == 0 {
            self.button.release();
        }
    }

    fn release(&mut self) {
        self.held_bindings = 0;
        self.button.release();
    }
}

/// Which actions are held, and which changed during the current frame.
///
/// An action is held while any of its bindings is held, so pressing a second
/// binding of a held action is not a new press. Asking for an action that
/// does not exist answers `false`; [`ActionMap::contains`] tells them apart.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Actions {
    actions: Vec<ActionState>,
}

impl Actions {
    /// The actions of `map`, all released.
    #[must_use]
    pub fn new(map: ActionMap) -> Self {
        let actions = map
            .into_actions()
            .into_iter()
            .map(|action| ActionState {
                action,
                held_bindings: 0,
                button: TrackedButton::UP,
            })
            .collect();
        Self { actions }
    }

    /// Whether the action `name` is held.
    #[must_use]
    pub fn is_down(&self, name: &str) -> bool {
        self.button(name).is_some_and(TrackedButton::is_down)
    }

    /// Whether the action `name` went down during the current frame.
    #[must_use]
    pub fn just_pressed(&self, name: &str) -> bool {
        self.button(name).is_some_and(TrackedButton::just_pressed)
    }

    /// Whether the action `name` went up during the current frame.
    #[must_use]
    pub fn just_released(&self, name: &str) -> bool {
        self.button(name).is_some_and(TrackedButton::just_released)
    }

    /// The bindings of the action `name`, if it exists.
    pub(crate) fn bindings(&self, name: &str) -> Option<&[Binding]> {
        self.actions
            .iter()
            .find(|state| state.action.name == name)
            .map(|state| state.action.bindings.as_slice())
    }

    /// Updates the actions bound to `binding`, which has just changed to `state`.
    pub(crate) fn binding_changed(&mut self, binding: Binding, state: ButtonState) {
        let bound = self
            .actions
            .iter_mut()
            .filter(|state| state.action.bindings.contains(&binding));
        for action in bound {
            match state {
                ButtonState::Pressed => action.binding_pressed(),
                ButtonState::Released => action.binding_released(),
            }
        }
    }

    pub(crate) fn release_all(&mut self) {
        for action in &mut self.actions {
            action.release();
        }
    }

    pub(crate) fn end_frame(&mut self) {
        for action in &mut self.actions {
            action.button.end_frame();
        }
    }

    fn button(&self, name: &str) -> Option<TrackedButton> {
        self.actions
            .iter()
            .find(|state| state.action.name == name)
            .map(|state| state.button)
    }
}

#[cfg(test)]
mod tests {
    use star_platform::{Key, MouseButton};

    use super::*;

    const JUMP: &str = "jump";
    const FIRE: &str = "fire";

    fn actions() -> Actions {
        let mut map = ActionMap::new();
        map.add(JUMP, &[Key::Space.into(), Key::W.into()]).unwrap();
        map.add(FIRE, &[MouseButton::Left.into(), Key::Space.into()])
            .unwrap();
        Actions::new(map)
    }

    #[test]
    fn a_binding_presses_its_action() {
        let mut actions = actions();

        actions.binding_changed(Key::W.into(), ButtonState::Pressed);

        assert!(actions.is_down(JUMP));
        assert!(actions.just_pressed(JUMP));
        assert!(!actions.is_down(FIRE));
    }

    #[test]
    fn a_binding_shared_by_two_actions_presses_both() {
        let mut actions = actions();

        actions.binding_changed(Key::Space.into(), ButtonState::Pressed);

        assert!(actions.is_down(JUMP) && actions.is_down(FIRE));
    }

    #[test]
    fn an_action_stays_held_while_any_binding_is_held() {
        let mut actions = actions();
        actions.binding_changed(Key::Space.into(), ButtonState::Pressed);
        actions.binding_changed(Key::W.into(), ButtonState::Pressed);
        actions.end_frame();

        actions.binding_changed(Key::Space.into(), ButtonState::Released);

        assert!(actions.is_down(JUMP));
        assert!(!actions.just_released(JUMP));

        actions.binding_changed(Key::W.into(), ButtonState::Released);

        assert!(!actions.is_down(JUMP));
        assert!(actions.just_released(JUMP));
    }

    #[test]
    fn a_second_binding_of_a_held_action_is_not_a_new_press() {
        let mut actions = actions();
        actions.binding_changed(Key::Space.into(), ButtonState::Pressed);
        actions.end_frame();

        actions.binding_changed(Key::W.into(), ButtonState::Pressed);

        assert!(!actions.just_pressed(JUMP));
    }

    #[test]
    fn end_frame_keeps_held_actions() {
        let mut actions = actions();
        actions.binding_changed(Key::W.into(), ButtonState::Pressed);

        actions.end_frame();

        assert!(actions.is_down(JUMP));
        assert!(!actions.just_pressed(JUMP));
    }

    #[test]
    fn release_all_releases_every_action() {
        let mut actions = actions();
        actions.binding_changed(Key::Space.into(), ButtonState::Pressed);
        actions.binding_changed(Key::W.into(), ButtonState::Pressed);

        actions.release_all();

        assert!(!actions.is_down(JUMP) && !actions.is_down(FIRE));

        actions.binding_changed(Key::W.into(), ButtonState::Pressed);
        actions.end_frame();
        actions.binding_changed(Key::W.into(), ButtonState::Released);

        assert!(!actions.is_down(JUMP), "no binding count is left over");
    }

    #[test]
    fn unknown_actions_are_never_held() {
        let actions = actions();

        assert!(!actions.is_down("dance"));
        assert!(!actions.just_pressed("dance"));
        assert!(!actions.just_released("dance"));
    }
}
