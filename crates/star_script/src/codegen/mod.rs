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
mod program;
mod statements;
mod writer;

use std::error::Error;
use std::fmt;

use crate::Span;
use crate::ast::{Expr, SourceFile};
use crate::semantic::{Analysis, ExprInfo, Resolution, Type};

/// Generates the Rust code of a checked program.
///
/// If one class has a `static func main()`, the code also gets a `main`
/// function that calls it, so it can be built as a program.
///
/// # Errors
///
/// Returns the first construct that cannot be translated yet.
pub fn generate(
    syntax: &SourceFile,
    analysis: &Analysis,
) -> std::result::Result<String, CodegenError> {
    program::generate(Context { syntax, analysis })
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
    /// Several classes declare `static func main()`.
    SeveralEntryPoints,
    /// A script class inherits from a class implemented by the engine.
    NativeBaseClass(String),
    /// The analysis lacks information it always provides for valid code:
    /// a bug in the compiler, not in the program.
    Internal(&'static str),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CodegenErrorKind::SeveralEntryPoints => {
                formatter.write_str("only one class can declare `static func main()`")
            }
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
}

#[cfg(test)]
mod tests;
