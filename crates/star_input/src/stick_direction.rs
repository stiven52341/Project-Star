//! Directions of the gamepad sticks, so a stick can drive actions such as
//! `"move_left"` as a key would.

use star_platform::GamepadAxis;

/// How far a stick must lean in a direction for it to count as held: half of
/// its travel, so a stick resting slightly off center never holds an action.
pub const STICK_PRESS_THRESHOLD: f32 = 0.5;

/// One way a gamepad stick can lean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StickDirection {
    /// Left stick, to the left.
    LeftStickLeft,
    /// Left stick, to the right.
    LeftStickRight,
    /// Left stick, up.
    LeftStickUp,
    /// Left stick, down.
    LeftStickDown,
    /// Right stick, to the left.
    RightStickLeft,
    /// Right stick, to the right.
    RightStickRight,
    /// Right stick, up.
    RightStickUp,
    /// Right stick, down.
    RightStickDown,
}

impl StickDirection {
    /// Every direction, in declaration order: `StickDirection::ALL[direction.index()] == direction`.
    pub const ALL: &'static [Self] = &[
        Self::LeftStickLeft,
        Self::LeftStickRight,
        Self::LeftStickUp,
        Self::LeftStickDown,
        Self::RightStickLeft,
        Self::RightStickRight,
        Self::RightStickUp,
        Self::RightStickDown,
    ];

    /// Number of directions.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the direction in [`StickDirection::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Name of the direction, as written after `Gamepad` in binding names.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::LeftStickLeft => "LeftStickLeft",
            Self::LeftStickRight => "LeftStickRight",
            Self::LeftStickUp => "LeftStickUp",
            Self::LeftStickDown => "LeftStickDown",
            Self::RightStickLeft => "RightStickLeft",
            Self::RightStickRight => "RightStickRight",
            Self::RightStickUp => "RightStickUp",
            Self::RightStickDown => "RightStickDown",
        }
    }

    /// The direction called `name` (see [`StickDirection::name`]).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|direction| direction.name() == name)
    }

    /// How far the stick leans this way, given the value of each axis
    /// (`value_of`): from `0.0` (centered, or leaning the other way) to `1.0`.
    #[must_use]
    pub fn strength(self, value_of: impl Fn(GamepadAxis) -> f32) -> f32 {
        let (axis, sign) = self.axis();
        (sign * value_of(axis)).clamp(0.0, 1.0)
    }

    /// The axis the direction is read from, and the sign of its values that
    /// lean this way (sticks have positive Y up).
    const fn axis(self) -> (GamepadAxis, f32) {
        match self {
            Self::LeftStickLeft => (GamepadAxis::LeftStickX, -1.0),
            Self::LeftStickRight => (GamepadAxis::LeftStickX, 1.0),
            Self::LeftStickUp => (GamepadAxis::LeftStickY, 1.0),
            Self::LeftStickDown => (GamepadAxis::LeftStickY, -1.0),
            Self::RightStickLeft => (GamepadAxis::RightStickX, -1.0),
            Self::RightStickRight => (GamepadAxis::RightStickX, 1.0),
            Self::RightStickUp => (GamepadAxis::RightStickY, 1.0),
            Self::RightStickDown => (GamepadAxis::RightStickY, -1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_direction_is_found_by_name_and_index() {
        for (index, direction) in StickDirection::ALL.iter().enumerate() {
            assert_eq!(direction.index(), index);
            assert_eq!(
                StickDirection::from_name(direction.name()),
                Some(*direction)
            );
        }
        assert_eq!(StickDirection::from_name("LeftStick"), None);
    }

    #[test]
    fn a_direction_only_counts_the_side_it_leans_to() {
        let leaning_left = |axis| {
            if axis == GamepadAxis::LeftStickX {
                -0.75
            } else {
                0.0
            }
        };

        assert!((StickDirection::LeftStickLeft.strength(leaning_left) - 0.75).abs() < f32::EPSILON);
        assert!(StickDirection::LeftStickRight.strength(leaning_left).abs() < f32::EPSILON);
        assert!(StickDirection::RightStickLeft.strength(leaning_left).abs() < f32::EPSILON);
    }
}
