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
//! For now it runs console programs: static functions, classes,
//! inheritance, decorators and coroutines (Phase 14 of `docs/roadmap.md`
//! brings the rest of the language step by step).

mod error;
mod lower;
mod run;
mod tree;
mod value;

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use star_script::CheckedSource;

pub use error::{LowerError, LowerErrorKind, RunError};

/// Where `Debug.print` writes its lines. It is shared, not borrowed: the
/// coroutines a program starts go on printing after the call that started
/// them, and the caller keeps its own handle to read what was written
/// (a buffer in tests, the console in the player).
pub type Output = Rc<RefCell<dyn Write>>;

/// A program ready to be interpreted. Cloning it costs nothing: the clones
/// share the same lowered code.
#[derive(Debug, Clone)]
pub struct Program {
    code: Rc<tree::Code>,
}

impl Program {
    /// Prepares a checked program to be run.
    ///
    /// # Errors
    ///
    /// When the program uses something the interpreter cannot run yet, or
    /// has no `static func main()` (or several).
    pub fn lower(checked: &CheckedSource) -> Result<Self, LowerError> {
        let code = lower::program(&checked.syntax, &checked.analysis)?;
        Ok(Self {
            code: Rc::new(code),
        })
    }

    /// Runs `static func main()`, and then the coroutines still waiting,
    /// with the clock moved by real time, until none is left (as a
    /// compiled console program does). `Debug.print` writes its lines to
    /// `output`.
    ///
    /// # Errors
    ///
    /// When the program stops with a runtime error of StarScript, or its
    /// output cannot be written.
    pub fn run(&self, output: &Output) -> Result<(), RunError> {
        let result = run::Machine::new(Rc::clone(&self.code), Rc::clone(output)).run();
        if result.is_err() {
            // The coroutines of a stopped program never go on.
            star_script_runtime::tasks::clear();
        }
        result
    }
}

#[cfg(test)]
mod tests;
