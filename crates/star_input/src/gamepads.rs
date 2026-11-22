//! Gamepad state: the connected gamepads, their buttons and analog controls.

use star_platform::{ButtonState, GamepadAxis, GamepadButton};

use crate::button_states::ButtonStates;

/// Most gamepads tracked at once; more are ignored. Eight is beyond what
/// local multiplayer games use.
pub const MAX_GAMEPADS: usize = 8;

/// Identifies a gamepad while it is connected. When it is disconnected, a
/// gamepad connected later may get the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamepadId(usize);

/// One connected gamepad.
#[derive(Debug, Clone, PartialEq)]
pub struct Gamepad {
    id: GamepadId,
    buttons: ButtonStates<{ GamepadButton::COUNT }>,
    axes: [f32; GamepadAxis::COUNT],
}

impl Gamepad {
    fn new(id: GamepadId) -> Self {
        Self {
            id,
            buttons: ButtonStates::new(),
            axes: [0.0; GamepadAxis::COUNT],
        }
    }

    /// Identifies this gamepad while it is connected.
    #[must_use]
    pub const fn id(&self) -> GamepadId {
        self.id
    }

    /// Whether `button` is held.
    #[must_use]
    pub const fn is_down(&self, button: GamepadButton) -> bool {
        self.buttons.get(button.index()).is_down()
    }

    /// Whether `button` went down during the current frame.
    #[must_use]
    pub const fn just_pressed(&self, button: GamepadButton) -> bool {
        self.buttons.get(button.index()).just_pressed()
    }

    /// Whether `button` went up during the current frame.
    #[must_use]
    pub const fn just_released(&self, button: GamepadButton) -> bool {
        self.buttons.get(button.index()).just_released()
    }

    /// Position of an analog control: sticks from `-1.0` to `1.0` (positive
    /// Y up), triggers from `0.0` to `1.0`. See [`GamepadAxis`].
    #[must_use]
    pub const fn axis(&self, axis: GamepadAxis) -> f32 {
        self.axes[axis.index()]
    }
}

/// The connected gamepads, in a fixed number of slots ([`MAX_GAMEPADS`]):
/// it never allocates memory in the game loop.
#[derive(Debug, Clone, PartialEq)]
pub struct Gamepads {
    slots: [Option<Gamepad>; MAX_GAMEPADS],
}

impl Gamepads {
    /// No gamepads connected.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [const { None }; MAX_GAMEPADS],
        }
    }

    /// Every connected gamepad.
    pub fn iter(&self) -> impl Iterator<Item = &Gamepad> {
        self.slots.iter().flatten()
    }

    /// The gamepad `id`, if it is connected.
    #[must_use]
    pub fn get(&self, id: GamepadId) -> Option<&Gamepad> {
        self.iter().find(|gamepad| gamepad.id == id)
    }

    /// Whether `button` is held on any connected gamepad.
    #[must_use]
    pub fn is_any_down(&self, button: GamepadButton) -> bool {
        self.iter().any(|gamepad| gamepad.is_down(button))
    }

    /// Connecting a gamepad that is already connected changes nothing.
    pub(crate) fn connect(&mut self, id: usize) {
        self.find_or_connect(GamepadId(id));
    }

    pub(crate) fn disconnect(&mut self, id: usize) {
        let id = GamepadId(id);
        for slot in &mut self.slots {
            if slot.as_ref().is_some_and(|gamepad| gamepad.id == id) {
                *slot = None;
            }
        }
    }

    /// Events can arrive from a gamepad whose connection was not reported;
    /// it is connected then.
    pub(crate) fn set_button(&mut self, id: usize, button: GamepadButton, state: ButtonState) {
        if let Some(gamepad) = self.find_or_connect(GamepadId(id)) {
            match state {
                ButtonState::Pressed => gamepad.buttons.press(button.index()),
                ButtonState::Released => gamepad.buttons.release(button.index()),
            };
        }
    }

    pub(crate) fn set_axis(&mut self, id: usize, axis: GamepadAxis, value: f32) {
        if let Some(gamepad) = self.find_or_connect(GamepadId(id)) {
            gamepad.axes[axis.index()] = value;
        }
    }

    pub(crate) fn end_frame(&mut self) {
        for gamepad in self.slots.iter_mut().flatten() {
            gamepad.buttons.end_frame();
        }
    }

    /// Which buttons are held on any gamepad, to tell when actions change.
    pub(crate) fn held_buttons(&self) -> [bool; GamepadButton::COUNT] {
        let mut held = [false; GamepadButton::COUNT];
        for gamepad in self.iter() {
            for (index, button) in GamepadButton::ALL.iter().enumerate() {
                held[index] |= gamepad.is_down(*button);
            }
        }
        held
    }

    /// `None` if the gamepad is new and every slot is taken.
    fn find_or_connect(&mut self, id: GamepadId) -> Option<&mut Gamepad> {
        let position = self
            .slots
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|gamepad| gamepad.id == id))
            .or_else(|| self.slots.iter().position(Option::is_none))?;
        Some(self.slots[position].get_or_insert_with(|| Gamepad::new(id)))
    }
}

impl Default for Gamepads {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: usize = 2;

    fn one_gamepad() -> Gamepads {
        let mut gamepads = Gamepads::new();
        gamepads.connect(PAD);
        gamepads
    }

    fn pad(gamepads: &Gamepads) -> &Gamepad {
        gamepads.get(GamepadId(PAD)).unwrap()
    }

    #[test]
    fn connected_gamepads_are_listed_once() {
        let mut gamepads = one_gamepad();

        gamepads.connect(PAD);

        assert_eq!(gamepads.iter().count(), 1);
        assert_eq!(pad(&gamepads).axis(GamepadAxis::LeftStickX), 0.0);
    }

    #[test]
    fn disconnected_gamepads_disappear() {
        let mut gamepads = one_gamepad();

        gamepads.disconnect(PAD);

        assert!(gamepads.get(GamepadId(PAD)).is_none());
    }

    #[test]
    fn buttons_are_tracked_per_frame() {
        let mut gamepads = one_gamepad();

        gamepads.set_button(PAD, GamepadButton::South, ButtonState::Pressed);

        assert!(pad(&gamepads).is_down(GamepadButton::South));
        assert!(pad(&gamepads).just_pressed(GamepadButton::South));

        gamepads.end_frame();

        assert!(!pad(&gamepads).just_pressed(GamepadButton::South));
        assert!(pad(&gamepads).is_down(GamepadButton::South));
    }

    #[test]
    fn axes_keep_their_last_value() {
        let mut gamepads = one_gamepad();

        gamepads.set_axis(PAD, GamepadAxis::LeftStickY, 0.75);
        gamepads.end_frame();

        assert_eq!(pad(&gamepads).axis(GamepadAxis::LeftStickY), 0.75);
    }

    #[test]
    fn events_from_an_unreported_gamepad_connect_it() {
        let mut gamepads = Gamepads::new();

        gamepads.set_button(PAD, GamepadButton::Start, ButtonState::Pressed);

        assert!(pad(&gamepads).is_down(GamepadButton::Start));
    }

    #[test]
    fn gamepads_beyond_the_capacity_are_ignored() {
        let mut gamepads = Gamepads::new();

        for id in 0..=MAX_GAMEPADS {
            gamepads.connect(id);
        }

        assert_eq!(gamepads.iter().count(), MAX_GAMEPADS);
        assert!(gamepads.get(GamepadId(MAX_GAMEPADS)).is_none());
    }

    #[test]
    fn held_buttons_combine_every_gamepad() {
        let mut gamepads = one_gamepad();
        gamepads.set_button(PAD, GamepadButton::East, ButtonState::Pressed);
        gamepads.set_button(PAD + 1, GamepadButton::West, ButtonState::Pressed);

        let held = gamepads.held_buttons();

        assert!(held[GamepadButton::East.index()] && held[GamepadButton::West.index()]);
        assert!(!held[GamepadButton::South.index()]);
        assert!(gamepads.is_any_down(GamepadButton::West));
    }
}
