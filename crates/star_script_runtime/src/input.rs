//! The state of the game's input, as StarScript's `Input` class sees it: its
//! actions and where the player points.
//!
//! `Input.is_down("jump")` is a static call: it has no object to read the
//! engine's input from. So the generated game copies the state of every
//! action, and the pointer, here at the start of each frame, and the calls
//! read it back. The game runs on one thread, so the state is kept per thread.

use std::cell::{Cell, RefCell};

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

/// Where the player points, in world units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointerState {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position, up.
    pub y: f32,
    /// Whether the player points somewhere this frame; if not, `x` and `y`
    /// are where they last did (the origin if they never did).
    pub is_present: bool,
}

struct Action {
    name: &'static str,
    state: ActionState,
}

thread_local! {
    static ACTIONS: RefCell<Vec<Action>> = const { RefCell::new(Vec::new()) };
    static POINTER: Cell<PointerState> = const {
        Cell::new(PointerState { x: 0.0, y: 0.0, is_present: false })
    };
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

/// Sets where the player points this frame, as `(x, y)` in world units, or
/// `None` if they point nowhere: then the last position is kept.
pub fn set_pointer(position: Option<(f32, f32)>) {
    POINTER.with(|pointer| {
        let last = pointer.get();
        pointer.set(match position {
            Some((x, y)) => PointerState {
                x,
                y,
                is_present: true,
            },
            None => PointerState {
                is_present: false,
                ..last
            },
        });
    });
}

/// Where the player points this frame.
#[must_use]
pub fn pointer() -> PointerState {
    POINTER.with(Cell::get)
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

    #[test]
    fn the_pointer_is_read_back() {
        set_pointer(Some((1.5, -2.0)));

        assert_eq!(
            pointer(),
            PointerState {
                x: 1.5,
                y: -2.0,
                is_present: true
            }
        );
    }

    #[test]
    fn a_missing_pointer_keeps_its_last_position() {
        set_pointer(Some((1.5, -2.0)));

        set_pointer(None);

        assert_eq!(
            pointer(),
            PointerState {
                x: 1.5,
                y: -2.0,
                is_present: false
            }
        );
    }
}
