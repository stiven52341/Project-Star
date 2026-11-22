//! Device-agnostic input for ProjectStar.
//!
//! [`Input`] holds what the player is doing this frame: held keys and
//! buttons, what changed since the last frame, the cursor and the wheel.
//! Abstract *actions* (e.g. "jump") will be built on top of it, so the same
//! game code works with keyboard, mouse, touch screens and gamepads.

mod button_states;
mod input;
mod keyboard;
mod mouse;

pub use input::Input;
pub use keyboard::Keyboard;
pub use mouse::Mouse;
pub use star_platform::{ButtonState, InputEvent, Key, MouseButton};
