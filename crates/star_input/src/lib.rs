//! Device-agnostic input for ProjectStar.
//!
//! [`Input`] holds what the player is doing this frame. Games ask for
//! abstract *actions* (e.g. `"jump"`), configured in an [`ActionMap`], so the
//! same game code works with keyboard, mouse, touch screens and gamepads.
//! The raw state of each device is also available.

mod action_map;
mod actions;
mod binding;
mod button_states;
mod gamepads;
mod input;
mod keyboard;
mod mouse;
mod stick_direction;
mod touches;

pub use action_map::ActionMap;
pub use actions::Actions;
pub use binding::Binding;
pub use gamepads::{Gamepad, GamepadId, Gamepads, MAX_GAMEPADS};
pub use input::Input;
pub use keyboard::Keyboard;
pub use mouse::Mouse;
pub use star_platform::{
    ButtonState, GamepadAxis, GamepadButton, InputEvent, Key, MouseButton, TouchPhase,
};
pub use stick_direction::{STICK_PRESS_THRESHOLD, StickDirection};
pub use touches::{MAX_TOUCHES, Touch, TouchId, Touches};
