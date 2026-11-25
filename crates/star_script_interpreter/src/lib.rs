//! Interpreter of StarScript: runs a checked program without building it.
//!
//! “Play” in the editor will run games with it, so they start at once;
//! building a project still compiles StarScript to Rust with `star_script`
//! (decision 64, rule 3.7 of `CLAUDE.md`). Both share the lexer, the
//! parser and the semantic analysis, and the rules that the generated Rust
//! takes from `star_script_runtime` (such as integer division), so a
//! program gives the same output run either way.
//!
//! It works in two stages:
//!
//! 1. [`Program::lower`] turns the syntax tree and its analysis into the
//!    interpreter's own tree: locals are numbered slots, calls point at
//!    their function and every conversion between number types is
//!    written out. What the interpreter cannot run yet is reported here,
//!    not halfway through running.
//! 2. [`Program::run`] walks that tree. StarScript's runtime errors, such
//!    as dividing an integer by zero, come back as a [`RunError`].
//!
//! For now it runs console programs made of static functions (Phase 14 of
//! `docs/roadmap.md` brings the rest of the language step by step).

mod error;
mod lower;
mod run;
mod tree;
mod value;

use std::io::Write;

use star_script::CheckedSource;

pub use error::{LowerError, LowerErrorKind, RunError};

/// A program ready to be interpreted.
#[derive(Debug)]
pub struct Program {
    functions: Vec<tree::Function>,
    main: tree::FunctionId,
}

impl Program {
    /// Prepares a checked program to be run.
    ///
    /// # Errors
    ///
    /// When the program uses something the interpreter cannot run yet, or
    /// has no `static func main()` (or several).
    pub fn lower(checked: &CheckedSource) -> Result<Self, LowerError> {
        lower::program(&checked.syntax, &checked.analysis)
    }

    /// Runs `static func main()`. `Debug.print` writes its lines to
    /// `output`.
    ///
    /// # Errors
    ///
    /// When the program stops with a runtime error of StarScript, or its
    /// output cannot be written.
    pub fn run(&self, output: &mut dyn Write) -> Result<(), RunError> {
        run::Machine::new(&self.functions, output)
            .call(self.main, Vec::new())
            .map(drop)
    }
}

#[cfg(test)]
mod tests;
