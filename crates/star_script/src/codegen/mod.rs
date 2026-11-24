//! Fourth compiler stage: turns a checked program into Rust code.
//!
//! The generated code is one Rust file meant to be the root of its own crate
//! (`main.rs` or `lib.rs`), which depends on `star_script_runtime`. Its shape
//! is described in `docs/starscript.md`, section 4. In short, for each class
//! `Player` it generates a module `classes::Player` with:
//!
//! - `Fields`: the fields the class declares.
//! - `Object`: a trait with its overridable methods; subclasses extend it.
//! - `Ref`: a shared reference to any object of the class or a subclass.
//! - `Methods`: the bodies of its methods and its constants.
//! - `Instance`: the concrete objects, with the fields of every ancestor.
//! - `init_` and `new`: construction in two phases, as the language requires.
//!
//! Generation cannot fail for a program that passed semantic analysis, except
//! for the few features listed in [`CodegenErrorKind`].

mod calls;
mod constructor;
mod conversions;
mod expressions;
mod names;
mod native;
mod native_3d;
mod program;
mod scene_fields;
mod statements;
mod writer;

use std::error::Error;
use std::fmt;

use crate::Span;
use crate::ast::{Expr, SourceFile};
use crate::semantic::{Analysis, ExprInfo, MethodInfo, Resolution, Type};

/// Generates the Rust code of a checked program.
///
/// The code gets a `main` function, so it can be built as a program, if it
/// has an entry point: a class with `static func main()`, or one game (a
/// class that extends `Game` and that no other class extends).
///
/// # Errors
///
/// Returns the first construct that cannot be translated yet, or a problem
/// with the entry point.
pub fn generate(
    syntax: &SourceFile,
    analysis: &Analysis,
) -> std::result::Result<GeneratedProgram, CodegenError> {
    program::generate(Context { syntax, analysis }, Entry::FromProgram)
}

/// Generates the scene player: a game with no classes of its own that
/// plays the scene named by its first argument, with its second argument
/// as the window's title (`player scenes/level.scene "My game"`). One build
/// of it plays any project without scripts. `syntax` and `analysis` are
/// those of an empty program.
///
/// # Errors
///
/// As [`generate`].
pub fn generate_player(
    syntax: &SourceFile,
    analysis: &Analysis,
) -> std::result::Result<GeneratedProgram, CodegenError> {
    program::generate(Context { syntax, analysis }, Entry::Player)
}

/// Where the generated program starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// Where its classes say: `main`, a game or the project's main scene.
    FromProgram,
    /// The scene player of [`generate_player`].
    Player,
}

/// The Rust code of a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedProgram {
    /// The code: the root file of a crate.
    pub code: String,
    /// How the program starts.
    pub entry_point: EntryPoint,
}

impl GeneratedProgram {
    /// Whether the code needs the engine (`star_engine`) besides the runtime.
    #[must_use]
    pub fn uses_engine(&self) -> bool {
        matches!(
            self.entry_point,
            EntryPoint::Game | EntryPoint::MainScene | EntryPoint::Player
        )
    }
}

/// How a generated program starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPoint {
    /// It does not start by itself: it can only be used as a library.
    None,
    /// It runs `static func main()` and ends: a command-line program.
    Main,
    /// It runs a game in a window of the engine.
    Game,
    /// It has no game class: it plays the project's main scene in a window
    /// of the engine.
    MainScene,
    /// It plays the scene its arguments name: the scene player of
    /// [`generate_player`].
    Player,
}

/// Why a program could not be translated to Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    /// What went wrong.
    pub kind: CodegenErrorKind,
    /// The code that could not be translated.
    pub span: Span,
}

/// The kinds of [`CodegenError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenErrorKind {
    /// More than one class could start the program.
    SeveralEntryPoints,
    /// The game class needs arguments to be created; the engine has none.
    GameNeedsArguments(String),
    /// A script class inherits from a class implemented by the engine.
    NativeBaseClass(String),
    /// The analysis lacks information it always provides for valid code:
    /// a bug in the compiler, not in the program.
    Internal(&'static str),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CodegenErrorKind::SeveralEntryPoints => formatter.write_str(
                "a program starts in one place: one class with `static func main()`, \
                 or one game (a class that extends `Game`)",
            ),
            CodegenErrorKind::GameNeedsArguments(game) => write!(
                formatter,
                "the game `{game}` must be creatable without arguments: its `init` cannot have parameters"
            ),
            CodegenErrorKind::NativeBaseClass(base) => write!(
                formatter,
                "classes cannot inherit from the engine class `{base}` yet"
            ),
            CodegenErrorKind::Internal(problem) => {
                write!(formatter, "internal compiler error: {problem}")
            }
        }
    }
}

impl Error for CodegenError {}

type Result<T> = std::result::Result<T, CodegenError>;

/// An internal error located at `span`.
const fn internal(problem: &'static str, span: Span) -> CodegenError {
    CodegenError {
        kind: CodegenErrorKind::Internal(problem),
        span,
    }
}

/// What every part of the generator reads: the program and its analysis.
#[derive(Clone, Copy)]
struct Context<'a> {
    syntax: &'a SourceFile,
    analysis: &'a Analysis,
}

impl<'a> Context<'a> {
    fn info(self, expression: &Expr) -> Result<&'a ExprInfo> {
        self.analysis
            .expression(expression)
            .ok_or_else(|| internal("expression without analysis", expression.span))
    }

    fn type_of(self, expression: &Expr) -> Result<&'a Type> {
        self.info(expression).map(|info| &info.ty)
    }

    fn resolution(self, expression: &Expr) -> Result<Resolution> {
        self.info(expression)?
            .resolution
            .ok_or_else(|| internal("name without resolution", expression.span))
    }

    /// Whether `callee` names an `async` method.
    fn calls_async_method(self, callee: &Expr) -> Result<bool> {
        Ok(match self.resolution(callee)? {
            Resolution::Method { class, index } => self
                .analysis
                .classes()
                .class(class)
                .methods
                .get(index)
                .is_some_and(MethodInfo::is_async),
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests;
