//! Runtime errors: what stops a game because it cannot continue correctly.

/// Stops the game with `message`, explained in StarScript terms.
///
/// This is the only place where the engine is allowed to panic (agreed
/// exception to the rule against `panic!`, see `CLAUDE.md`): the errors that
/// reach it, such as reading past the end of a list or dividing an integer by
/// zero, cannot be detected when compiling and leave no correct way to go on.
/// Stopping with a clear message is safer than continuing with a made-up value.
///
/// # Panics
///
/// Always: that is how the game stops.
#[cold]
#[track_caller]
pub fn fault(message: &str) -> ! {
    #[allow(clippy::panic)]
    {
        panic!("StarScript runtime error: {message}")
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
