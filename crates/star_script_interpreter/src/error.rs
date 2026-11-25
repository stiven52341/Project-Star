//! Why a program cannot be prepared, or stopped while running.

use std::{fmt, io};

use star_script::Span;
use star_script_runtime::RuntimeError;

/// Why a program could not be prepared to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    /// What went wrong.
    pub kind: LowerErrorKind,
    /// The code it is about.
    pub span: Span,
}

/// The kinds of [`LowerError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerErrorKind {
    /// A part of StarScript the interpreter cannot run yet.
    Unsupported(&'static str),
    /// No class has a `static func main()` or extends `Game`.
    NoEntryPoint,
    /// More than one class has a `static func main()`, or extends `Game`
    /// without being extended, or both.
    SeveralEntryPoints,
    /// The game's class needs arguments to be created.
    GameNeedsArguments,
    /// The analysis does not describe the program as expected: a bug of
    /// the interpreter or of the analysis, not of the program.
    Internal(&'static str),
}

impl LowerError {
    pub(crate) const fn new(kind: LowerErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub(crate) const fn unsupported(what: &'static str, span: Span) -> Self {
        Self::new(LowerErrorKind::Unsupported(what), span)
    }

    pub(crate) const fn internal(what: &'static str, span: Span) -> Self {
        Self::new(LowerErrorKind::Internal(what), span)
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LowerErrorKind::Unsupported(what) => {
                write!(formatter, "the interpreter cannot run {what} yet")
            }
            LowerErrorKind::NoEntryPoint => {
                formatter.write_str("the program has no `static func main()` and no game class")
            }
            LowerErrorKind::SeveralEntryPoints => formatter
                .write_str("the program has more than one `static func main()` or game class"),
            LowerErrorKind::GameNeedsArguments => formatter.write_str(
                "the game must be creatable without arguments: its `init` cannot have parameters",
            ),
            LowerErrorKind::Internal(what) => {
                write!(formatter, "internal interpreter error: {what}")
            }
        }
    }
}

impl std::error::Error for LowerError {}

/// Why a running program stopped before its end.
#[derive(Debug)]
pub enum RunError {
    /// A runtime error of StarScript, the same a compiled game stops with.
    Fault(RuntimeError),
    /// What the program printed could not be written.
    Output(io::Error),
    /// The program was run as what it is not: a game as a console
    /// program, or the other way round. It holds what the program is.
    WrongKind(crate::ProgramKind),
    /// The interpreter's tree did not hold what running expected: a bug of
    /// the interpreter, not of the program.
    Internal(&'static str),
}

impl From<RuntimeError> for RunError {
    fn from(error: RuntimeError) -> Self {
        Self::Fault(error)
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fault(error) => error.fmt(formatter),
            Self::Output(error) => write!(formatter, "cannot write the program's output: {error}"),
            Self::WrongKind(crate::ProgramKind::Game) => {
                formatter.write_str("the program is a game: play it in a window")
            }
            Self::WrongKind(crate::ProgramKind::Console) => {
                formatter.write_str("the program is a console program, not a game")
            }
            Self::Internal(what) => write!(formatter, "internal interpreter error: {what}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fault(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::WrongKind(_) | Self::Internal(_) => None,
        }
    }
}
