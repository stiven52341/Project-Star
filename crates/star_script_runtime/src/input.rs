//! The state of the game's input actions, as StarScript's `Input` class sees it.
//!
//! `Input.is_down("jump")` is a static call: it has no object to read the
//! engine's input from. So the generated game copies the state of every
//! action here at the start of each frame, and the calls read it back. The
//! game runs on one thread, so the state is kept per thread.

use std::cell::RefCell;

/// The state of one action during the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionState {
    /// Whether the action is held.
    pub is_down: bool,
    /// Whether the action went down during the current frame.
    pub just_pressed: bool,
    /// Whether the action went up during the current frame.
    pub just_released: bool,
}

struct Action {
    name: &'static str,
    state: ActionState,
}

thread_local! {
    static ACTIONS: RefCell<Vec<Action>> = const { RefCell::new(Vec::new()) };
}

/// Sets the actions of the game, all released. Called once, when it starts.
pub fn declare_actions(names: &[&'static str]) {
    ACTIONS.with(|actions| {
        if let Ok(mut actions) = actions.try_borrow_mut() {
            *actions = names
                .iter()
                .map(|name| Action {
                    name,
                    state: ActionState::default(),
                })
                .collect();
        }
    });
}

/// Sets the state of the action at `index` in the declared list. Called by
/// the game every frame; it never allocates memory.
pub fn set_action_state(index: usize, state: ActionState) {
    ACTIONS.with(|actions| {
        if let Ok(mut actions) = actions.try_borrow_mut()
            && let Some(action) = actions.get_mut(index)
        {
            action.state = state;
        }
    });
}

/// The state of the action `name`; an action that was not declared is never held.
#[must_use]
pub fn action_state(name: &str) -> ActionState {
    ACTIONS.with(|actions| {
        actions
            .try_borrow()
            .ok()
            .and_then(|actions| {
                actions
                    .iter()
                    .find(|action| action.name == name)
                    .map(|action| action.state)
            })
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELD: ActionState = ActionState {
        is_down: true,
        just_pressed: true,
        just_released: false,
    };

    #[test]
    fn declared_actions_start_released() {
        declare_actions(&["jump"]);

        assert_eq!(action_state("jump"), ActionState::default());
    }

    #[test]
    fn states_are_read_back_by_name() {
        declare_actions(&["jump", "fire"]);

        set_action_state(1, HELD);

        assert_eq!(action_state("fire"), HELD);
        assert_eq!(action_state("jump"), ActionState::default());
    }

    #[test]
    fn unknown_actions_and_indexes_are_ignored() {
        declare_actions(&["jump"]);

        set_action_state(5, HELD);

        assert_eq!(action_state("dance"), ActionState::default());
    }

    #[test]
    fn declaring_again_replaces_the_actions() {
        declare_actions(&["jump"]);
        set_action_state(0, HELD);

        declare_actions(&["fire"]);

        assert_eq!(action_state("jump"), ActionState::default());
    }
}
