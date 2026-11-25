//! Runtime errors: what stops a game because it cannot continue correctly.

use std::cell::Cell;
use std::fmt;

thread_local! {
    /// Whether this thread runs `parallel` functions, whose faults stop the
    /// game from its own thread, when their result is awaited.
    static ON_WORKER: Cell<bool> = const { Cell::new(false) };
}

/// A fault on a thread of `parallel` functions, on its way to the game's
/// thread.
pub(crate) struct WorkerFault(pub(crate) String);

/// Marks the current thread as one that runs `parallel` functions.
pub(crate) fn mark_worker_thread() {
    ON_WORKER.with(|on_worker| on_worker.set(true));
}

/// A StarScript error found while running, such as dividing an integer by
/// zero, before it stops anything: the interpreter returns it, and the
/// generated Rust hands its message to [`fault`]. Both give the same text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    /// An error explained by `message`, in StarScript terms.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// What went wrong, without the prefix that [`fault`] adds.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Stops the game with this error (see [`fault`]).
    #[cold]
    #[track_caller]
    pub fn stop(&self) -> ! {
        fault(&self.message)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{FAULT_PREFIX}{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// What every runtime error starts with, compiled or interpreted.
const FAULT_PREFIX: &str = "StarScript runtime error: ";

/// Stops the game with `message`, explained in StarScript terms.
///
/// This is the only place where the engine is allowed to panic (agreed
/// exception to the rule against `panic!`, see `CLAUDE.md`): the errors that
/// reach it, such as reading past the end of a list or dividing an integer by
/// zero, cannot be detected when compiling and leave no correct way to go on.
/// Stopping with a clear message is safer than continuing with a made-up value.
///
/// On a thread of `parallel` functions it only stops the work, silently,
/// so the game's thread reports it once, when it awaits the result.
///
/// # Panics
///
/// Always: that is how the game stops.
#[cold]
#[track_caller]
pub fn fault(message: &str) -> ! {
    if ON_WORKER.with(Cell::get) {
        // Unwinds without the panic message, which the game's thread gives.
        std::panic::resume_unwind(Box::new(WorkerFault(message.to_owned())));
    }
    #[allow(clippy::panic)]
    {
        panic!("{FAULT_PREFIX}{message}")
    }
}

/// The value of something the compiler proved is never `null` here.
///
/// The compiler only calls this after a check such as `if x != null`, so a
/// `None` would mean a bug in the compiler itself, not in the game.
#[track_caller]
pub fn proven<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => fault("internal compiler error: a value proven not to be null is null"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_errors_read_as_faults() {
        let error = RuntimeError::new("boom");
        assert_eq!(error.message(), "boom");
        assert_eq!(error.to_string(), "StarScript runtime error: boom");
    }

    #[test]
    fn proven_values_are_returned() {
        assert_eq!(proven(Some(3)), 3);
    }

    #[test]
    #[should_panic(expected = "StarScript runtime error: internal compiler error")]
    fn proven_null_is_a_fault() {
        proven::<i64>(None);
    }

    #[test]
    #[should_panic(expected = "StarScript runtime error: boom")]
    fn faults_explain_the_problem() {
        fault("boom");
    }
}
