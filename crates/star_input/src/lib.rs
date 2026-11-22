//! Device-agnostic input for ProjectStar.
//!
//! [`Input`] holds what the player is doing this frame. Games ask for
//! abstract *actions* (e.g. `"jump"`), configured in an [`ActionMap`], so the
//! same game code works with keyboard, mouse and, later, touch screens and
//! gamepads. Raw keyboard and mouse state is also available.

mod action_map;
mod actions;
mod binding;
mod button_states;
mod input;
mod keyboard;
mod mouse;

pub use action_map::ActionMap;
pub use actions::Actions;
pub use binding::Binding;
pub use input::Input;
pub use keyboard::Keyboard;
pub use mouse::Mouse;
pub use star_platform::{ButtonState, InputEvent, Key, MouseButton};
