//! The engine's input, copied where StarScript's `Input` reads it.

use star_engine::star_input::Input;
use star_engine::star_platform::WindowSize;
use star_engine::star_render::Camera;
use star_script_runtime::input::{ActionState, set_action_state, set_pointer};

/// Copies where the player points, in world units, where `Input` reads it.
/// `view` is the camera the last frame was drawn from: what the player saw.
pub fn share_pointer(input: &Input, view: &Camera, window: WindowSize) {
    let position = input
        .pointer()
        .and_then(|pixel| view.screen_to_world_2d(pixel, window));
    set_pointer(position.map(|point| (point.x, point.y)));
}

/// Copies the state of each action, in the order of `names` (the order
/// they were declared to the runtime), where `Input` reads it.
pub fn share_actions<Name: AsRef<str>>(input: &Input, names: &[Name]) {
    let actions = input.actions();
    for (index, name) in names.iter().enumerate() {
        let name = name.as_ref();
        let state = ActionState {
            is_down: actions.is_down(name),
            just_pressed: actions.just_pressed(name),
            just_released: actions.just_released(name),
            strength: input.action_strength(name),
        };
        set_action_state(index, state);
    }
}

#[cfg(test)]
mod tests {
    use star_script_runtime::input::{action_state, declare_actions, pointer};

    use super::*;

    #[test]
    fn actions_not_pressed_are_shared_as_up() {
        declare_actions(&["jump"]);
        set_action_state(
            0,
            ActionState {
                is_down: true,
                just_pressed: true,
                just_released: false,
                strength: 1.0,
            },
        );

        share_actions(&Input::new(), &["jump"]);

        let jump = action_state("jump");
        assert!(!jump.is_down);
        assert!(!jump.just_pressed);
        assert!(jump.strength.abs() < f32::EPSILON);
    }

    #[test]
    fn no_pointer_is_shared_as_absent() {
        share_pointer(&Input::new(), &Camera::default(), WindowSize::new(800, 600));

        assert!(!pointer().is_present);
    }
}
