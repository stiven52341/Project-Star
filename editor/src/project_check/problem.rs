//! A mistake found while checking a project.

use std::fmt;

use star_blueprint::Origin;
use star_script::Position;

/// A mistake, and the file (and place in it, when known) where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Problem {
    /// The file, relative to the project's folder: `assets/player.star`.
    pub(crate) file: String,
    /// Line and column, for mistakes in code.
    pub(crate) position: Option<Position>,
    /// What is wrong.
    pub(crate) message: String,
    /// For mistakes in blueprints, the part of the blueprint they are in.
    pub(crate) origin: Option<Origin>,
}

impl Problem {
    /// A mistake at `position` of `file`.
    pub(crate) fn at(file: impl Into<String>, position: Position, message: String) -> Self {
        Self {
            file: file.into(),
            position: Some(position),
            message,
            origin: None,
        }
    }

    /// A mistake about the whole of `file`.
    pub(crate) fn in_file(file: impl Into<String>, message: String) -> Self {
        Self {
            file: file.into(),
            position: None,
            message,
            origin: None,
        }
    }

    /// The same mistake, placed on `origin` of its blueprint.
    pub(crate) fn placed_on(mut self, origin: Option<Origin>) -> Self {
        self.origin = origin;
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Some(position) => write!(
                formatter,
                "{}:{}:{}: {}",
                self.file, position.line, position.column, self.message
            ),
            None => write!(formatter, "{}: {}", self.file, self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_in_code_show_their_line_and_column() {
        let problem = Problem::at(
            "assets/a.star",
            Position { line: 2, column: 5 },
            "unknown name `x`".to_owned(),
        );

        assert_eq!(problem.to_string(), "assets/a.star:2:5: unknown name `x`");
    }

    #[test]
    fn problems_with_a_whole_file_show_only_the_file() {
        let problem = Problem::in_file("project.toml", "bad".to_owned());

        assert_eq!(problem.to_string(), "project.toml: bad");
    }
}
