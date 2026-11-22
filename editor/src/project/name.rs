//! Validated project name.

use std::fmt;

/// Longest name accepted, in characters; keeps folder paths reasonable.
pub(crate) const MAX_NAME_LENGTH: usize = 64;

/// Names Windows reserves for devices; folders cannot use them.
const WINDOWS_RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Why a text cannot be used as a project name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectNameError {
    /// The name is empty or only whitespace.
    Empty,
    /// The name has more than [`MAX_NAME_LENGTH`] characters.
    TooLong,
    /// The name does not start with a letter or a digit.
    InvalidStart,
    /// The name contains a character other than letters, digits, spaces, `-` or `_`.
    InvalidCharacter(char),
    /// The name is reserved by an operating system.
    Reserved,
}

impl fmt::Display for ProjectNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("the project name is empty"),
            Self::TooLong => write!(
                formatter,
                "the project name is longer than {MAX_NAME_LENGTH} characters"
            ),
            Self::InvalidStart => {
                formatter.write_str("the project name must start with a letter or a digit")
            }
            Self::InvalidCharacter(character) => {
                write!(formatter, "the project name cannot contain '{character}'")
            }
            Self::Reserved => formatter.write_str("the project name is reserved by the system"),
        }
    }
}

impl std::error::Error for ProjectNameError {}

/// A project name that is also a valid folder name on every desktop OS.
///
/// Only letters, digits, spaces, `-` and `_` are allowed, so the name can be
/// written to files without escaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectName(String);

impl ProjectName {
    /// Validates `text`, ignoring leading and trailing whitespace.
    pub(crate) fn parse(text: &str) -> Result<Self, ProjectNameError> {
        let name = text.trim();
        let first = name.chars().next().ok_or(ProjectNameError::Empty)?;
        if name.chars().count() > MAX_NAME_LENGTH {
            return Err(ProjectNameError::TooLong);
        }
        if !first.is_alphanumeric() {
            return Err(ProjectNameError::InvalidStart);
        }
        if let Some(invalid) = name.chars().find(|&character| !is_allowed(character)) {
            return Err(ProjectNameError::InvalidCharacter(invalid));
        }
        if is_reserved(name) {
            return Err(ProjectNameError::Reserved);
        }
        Ok(Self(name.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_allowed(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, ' ' | '-' | '_')
}

fn is_reserved(name: &str) -> bool {
    WINDOWS_RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_letters_digits_spaces_dashes_and_underscores() {
        let name = ProjectName::parse("Mi Juego_2-final").expect("valid name");

        assert_eq!(name.as_str(), "Mi Juego_2-final");
    }

    #[test]
    fn accepts_accented_letters() {
        assert!(ProjectName::parse("Canción Ñandú").is_ok());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let name = ProjectName::parse("  Space Game  ").expect("valid name");

        assert_eq!(name.as_str(), "Space Game");
    }

    #[test]
    fn rejects_blank_names() {
        assert_eq!(ProjectName::parse("   "), Err(ProjectNameError::Empty));
    }

    #[test]
    fn rejects_names_that_are_too_long() {
        let long_name = "a".repeat(MAX_NAME_LENGTH + 1);

        assert_eq!(
            ProjectName::parse(&long_name),
            Err(ProjectNameError::TooLong)
        );
    }

    #[test]
    fn rejects_names_starting_with_a_symbol() {
        assert_eq!(
            ProjectName::parse("_game"),
            Err(ProjectNameError::InvalidStart)
        );
    }

    #[test]
    fn rejects_path_separators_and_quotes() {
        assert_eq!(
            ProjectName::parse("my/game"),
            Err(ProjectNameError::InvalidCharacter('/'))
        );
        assert_eq!(
            ProjectName::parse("my\"game"),
            Err(ProjectNameError::InvalidCharacter('"'))
        );
    }

    #[test]
    fn rejects_windows_device_names_in_any_case() {
        assert_eq!(ProjectName::parse("con"), Err(ProjectNameError::Reserved));
    }
}
