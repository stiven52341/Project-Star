//! How deep calls are nested: a function that calls itself without end
//! stops the game with a runtime error, instead of overflowing the
//! thread's stack, which ends the program without a word.
//!
//! Every call of a function that is not `async` counts, compiled or
//! interpreted, so both stop at the same depth with the same message.
//! `async` methods are left out: a coroutine waits between frames, so its
//! calls are not nested in the code that started it.

use std::cell::Cell;

use crate::RuntimeError;

/// How many calls may be nested: deeper ones are taken to never end. As
/// many as several languages allow by default; the interpreter fits them in
/// the stack of a desktop's main thread.
pub const MAX_CALL_DEPTH: u32 = 1000;

thread_local! {
    /// The calls going on in this thread.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// A call going on: while it lives, it counts as one level of nesting.
#[must_use = "the call only counts while its guard lives"]
#[derive(Debug)]
pub struct CallGuard(());

impl CallGuard {
    /// Counts a call that begins, or gives the error of one nested too deep
    /// (and does not count it).
    ///
    /// # Errors
    ///
    /// [`MAX_CALL_DEPTH`] calls are already going on.
    pub fn enter() -> Result<Self, RuntimeError> {
        DEPTH.with(|depth| {
            if depth.get() >= MAX_CALL_DEPTH {
                return Err(too_deep());
            }
            depth.set(depth.get() + 1);
            Ok(Self(()))
        })
    }
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// [`CallGuard::enter`] for the generated Rust: a call nested too deep
/// stops the game.
pub fn enter() -> CallGuard {
    CallGuard::enter().unwrap_or_else(|error| error.stop())
}

fn too_deep() -> RuntimeError {
    RuntimeError::new(format!(
        "more than {MAX_CALL_DEPTH} calls nested: a function probably calls itself without end"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn depth() -> u32 {
        DEPTH.with(Cell::get)
    }

    #[test]
    fn calls_count_while_they_go_on() {
        let before = depth();
        {
            let _outer = enter();
            let _inner = enter();
            assert_eq!(depth(), before + 2);
        }
        assert_eq!(depth(), before);
    }

    #[test]
    fn calls_past_the_limit_are_an_error_and_not_counted() {
        let guards: Vec<CallGuard> = (depth()..MAX_CALL_DEPTH).map(|_| enter()).collect();

        let error = CallGuard::enter().expect_err("one call too many");

        assert_eq!(
            error.message(),
            "more than 1000 calls nested: a function probably calls itself without end"
        );
        assert_eq!(depth(), MAX_CALL_DEPTH);
        drop(guards);
        assert!(CallGuard::enter().is_ok());
    }
}
