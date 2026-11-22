//! Raw input events, in the engine's own vocabulary.
//!
//! The windowing library's types never leave this crate: these events are
//! what the rest of the engine (`star_input`) builds its input state from.

/// Whether a key or button went down or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// The key or button was pressed.
    Pressed,
    /// The key or button was released.
    Released,
}

/// Declares [`Key`] together with the list of all its variants, so the list
/// can never get out of sync with the enum.
macro_rules! define_keys {
    ($($name:ident),+ $(,)?) => {
        /// A key, identified by its **physical position** on a US QWERTY
        /// keyboard, whatever the user's layout.
        ///
        /// So `Key::W` is the key above `Key::S` also on an AZERTY keyboard,
        /// where it shows a «Z»: movement controls stay in the same place.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            $(
                #[doc = concat!("The `", stringify!($name), "` key.")]
                $name,
            )+
        }

        impl Key {
            /// Every key, in declaration order: `Key::ALL[key.index()] == key`.
            pub const ALL: &'static [Self] = &[$(Self::$name),+];

            /// The name of the key, as written in settings files: `"Space"`, `"A"`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name),)+
                }
            }
        }
    };
}

define_keys! {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Space, Enter, Escape, Tab, Backspace, Delete, Insert,
    Home, End, PageUp, PageDown, CapsLock,
    ShiftLeft, ShiftRight, ControlLeft, ControlRight,
    AltLeft, AltRight, SuperLeft, SuperRight,
    Minus, Equal, BracketLeft, BracketRight, Backslash,
    Semicolon, Quote, Backquote, Comma, Period, Slash,
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide,
    NumpadDecimal, NumpadEnter,
}

impl Key {
    /// Number of keys.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the key in [`Key::ALL`], to store per-key state in arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The key called `name` (see [`Key::name`]).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|key| key.name() == name)
    }
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// The main button, usually the left one.
    Left,
    /// The secondary button, usually the right one.
    Right,
    /// The wheel button.
    Middle,
    /// The side button that goes back in browsers.
    Back,
    /// The side button that goes forward in browsers.
    Forward,
}

impl MouseButton {
    /// Every button, in declaration order: `MouseButton::ALL[button.index()] == button`.
    pub const ALL: &'static [Self] = &[
        Self::Left,
        Self::Right,
        Self::Middle,
        Self::Back,
        Self::Forward,
    ];

    /// Number of buttons.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the button in [`MouseButton::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The name of the button: `"Left"`, `"Right"`…
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Middle => "Middle",
            Self::Back => "Back",
            Self::Forward => "Forward",
        }
    }

    /// The button called `name` (see [`MouseButton::name`]).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|button| button.name() == name)
    }
}

/// A gamepad button, named by its **position** on the gamepad, so the same
/// code works with every brand: `South` is «A» on Xbox and «✕» on PlayStation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    /// Bottom face button.
    South,
    /// Right face button.
    East,
    /// Top face button.
    North,
    /// Left face button.
    West,
    /// Left shoulder button (LB, L1).
    LeftBumper,
    /// Right shoulder button (RB, R1).
    RightBumper,
    /// Left trigger (LT, L2) pressed far enough; its travel is [`GamepadAxis::LeftTrigger`].
    LeftTrigger,
    /// Right trigger (RT, R2) pressed far enough; its travel is [`GamepadAxis::RightTrigger`].
    RightTrigger,
    /// Left menu button (Back, View, Share, Select).
    Select,
    /// Right menu button (Start, Menu, Options).
    Start,
    /// Central button with the brand's logo.
    Mode,
    /// Pressing the left stick (L3).
    LeftStick,
    /// Pressing the right stick (R3).
    RightStick,
    /// Directional pad, up.
    DPadUp,
    /// Directional pad, down.
    DPadDown,
    /// Directional pad, left.
    DPadLeft,
    /// Directional pad, right.
    DPadRight,
}

impl GamepadButton {
    /// Every button, in declaration order: `GamepadButton::ALL[button.index()] == button`.
    pub const ALL: &'static [Self] = &[
        Self::South,
        Self::East,
        Self::North,
        Self::West,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::Select,
        Self::Start,
        Self::Mode,
        Self::LeftStick,
        Self::RightStick,
        Self::DPadUp,
        Self::DPadDown,
        Self::DPadLeft,
        Self::DPadRight,
    ];

    /// Number of buttons.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the button in [`GamepadButton::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The name of the button: `"South"`, `"LeftBumper"`…
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::South => "South",
            Self::East => "East",
            Self::North => "North",
            Self::West => "West",
            Self::LeftBumper => "LeftBumper",
            Self::RightBumper => "RightBumper",
            Self::LeftTrigger => "LeftTrigger",
            Self::RightTrigger => "RightTrigger",
            Self::Select => "Select",
            Self::Start => "Start",
            Self::Mode => "Mode",
            Self::LeftStick => "LeftStick",
            Self::RightStick => "RightStick",
            Self::DPadUp => "DPadUp",
            Self::DPadDown => "DPadDown",
            Self::DPadLeft => "DPadLeft",
            Self::DPadRight => "DPadRight",
        }
    }

    /// The button called `name` (see [`GamepadButton::name`]).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|button| button.name() == name)
    }
}

/// An analog control of a gamepad.
///
/// Sticks go from `-1.0` to `1.0` (positive X to the right, positive Y
/// **up**) and rest at `0.0`; small movements around the center are already
/// ignored (dead zone). Triggers go from `0.0` (released) to `1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    /// Left stick, horizontal.
    LeftStickX,
    /// Left stick, vertical.
    LeftStickY,
    /// Right stick, horizontal.
    RightStickX,
    /// Right stick, vertical.
    RightStickY,
    /// Left trigger travel.
    LeftTrigger,
    /// Right trigger travel.
    RightTrigger,
}

impl GamepadAxis {
    /// Every axis, in declaration order: `GamepadAxis::ALL[axis.index()] == axis`.
    pub const ALL: &'static [Self] = &[
        Self::LeftStickX,
        Self::LeftStickY,
        Self::RightStickX,
        Self::RightStickY,
        Self::LeftTrigger,
        Self::RightTrigger,
    ];

    /// Number of axes.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the axis in [`GamepadAxis::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Stage of a finger's contact with a touch screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    /// The finger touched the screen.
    Started,
    /// The finger moved while touching the screen.
    Moved,
    /// The finger left the screen.
    Ended,
    /// The system took over the touch (e.g. for a gesture); the finger no
    /// longer belongs to the game.
    Cancelled,
}

/// Something the user did with an input device.
///
/// Positions are in physical pixels of the window's drawable area, with the
/// origin at the top-left corner and Y growing downwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// A key went down or up. Repeats while a key is held are not reported.
    Key {
        /// The physical key.
        key: Key,
        /// Whether it went down or up.
        state: ButtonState,
    },
    /// A mouse button went down or up.
    MouseButton {
        /// The button.
        button: MouseButton,
        /// Whether it went down or up.
        state: ButtonState,
    },
    /// The cursor moved inside the window.
    CursorMoved {
        /// Horizontal position, in pixels from the left edge.
        x: f32,
        /// Vertical position, in pixels from the top edge.
        y: f32,
    },
    /// The cursor left the window.
    CursorLeft,
    /// The mouse wheel (or a touchpad) scrolled, in lines.
    MouseWheel {
        /// Horizontal lines; positive to the right.
        x: f32,
        /// Vertical lines; positive when the wheel turns away from the user.
        y: f32,
    },
    /// A finger touched, moved on or left a touch screen.
    Touch {
        /// Identifies the finger while it touches the screen; it may be
        /// reused by a later touch.
        id: u64,
        /// What the finger did.
        phase: TouchPhase,
        /// Horizontal position, in pixels from the left edge.
        x: f32,
        /// Vertical position, in pixels from the top edge.
        y: f32,
    },
    /// A gamepad was connected (also reported for those connected at start).
    GamepadConnected {
        /// Identifies the gamepad while it is connected.
        gamepad: usize,
    },
    /// A gamepad was disconnected.
    GamepadDisconnected {
        /// The gamepad.
        gamepad: usize,
    },
    /// A gamepad button went down or up.
    GamepadButton {
        /// The gamepad.
        gamepad: usize,
        /// The button.
        button: GamepadButton,
        /// Whether it went down or up.
        state: ButtonState,
    },
    /// An analog control of a gamepad moved.
    GamepadAxis {
        /// The gamepad.
        gamepad: usize,
        /// The control.
        axis: GamepadAxis,
        /// Its new value (see [`GamepadAxis`] for the ranges).
        value: f32,
    },
    /// The window stopped receiving input. Keys and buttons held at that moment
    /// will not report their release, so they must be treated as released.
    FocusLost,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_at_its_own_index() {
        for (index, key) in Key::ALL.iter().enumerate() {
            assert_eq!(key.index(), index);
        }
    }

    #[test]
    fn every_mouse_button_is_at_its_own_index() {
        for (index, button) in MouseButton::ALL.iter().enumerate() {
            assert_eq!(button.index(), index);
        }
    }

    #[test]
    fn every_gamepad_control_is_at_its_own_index() {
        for (index, button) in GamepadButton::ALL.iter().enumerate() {
            assert_eq!(button.index(), index);
        }
        for (index, axis) in GamepadAxis::ALL.iter().enumerate() {
            assert_eq!(axis.index(), index);
        }
    }

    #[test]
    fn names_lead_back_to_their_control() {
        for key in Key::ALL {
            assert_eq!(Key::from_name(key.name()), Some(*key));
        }
        for button in MouseButton::ALL {
            assert_eq!(MouseButton::from_name(button.name()), Some(*button));
        }
        for button in GamepadButton::ALL {
            assert_eq!(GamepadButton::from_name(button.name()), Some(*button));
        }
        assert_eq!(Key::Space.name(), "Space");
        assert_eq!(Key::from_name("space"), None, "names are case-sensitive");
    }

    #[test]
    fn the_last_key_fits_in_the_count() {
        assert_eq!(Key::NumpadEnter.index(), Key::COUNT - 1);
        assert_eq!(MouseButton::Forward.index(), MouseButton::COUNT - 1);
        assert_eq!(GamepadButton::DPadRight.index(), GamepadButton::COUNT - 1);
        assert_eq!(GamepadAxis::RightTrigger.index(), GamepadAxis::COUNT - 1);
    }
}
