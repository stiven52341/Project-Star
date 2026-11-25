//! A game being played (decision 70). The runtime's `Host` functions,
//! such as running a node's `update`, take no interpreter, so the program
//! of the game is kept per thread while it plays, as the runtime keeps its
//! coroutines and input.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::RunError;
use crate::host::GameObject;
use crate::run::{Hook, Machine};
use crate::value::Value;

thread_local! {
    /// The game this thread plays, if any.
    static PLAYING: RefCell<Option<Playing>> = const { RefCell::new(None) };
}

/// A game and the program that runs its code.
#[derive(Clone)]
struct Playing {
    machine: Machine,
    game: Rc<GameObject>,
}

/// Starts playing `game`, whose code `machine` runs.
pub(crate) fn begin(machine: Machine, game: Rc<GameObject>) {
    PLAYING.with(|playing| {
        if let Ok(mut playing) = playing.try_borrow_mut() {
            *playing = Some(Playing { machine, game });
        }
    });
}

/// Stops playing; the runtime error that closed the game, if one did.
pub(crate) fn end() -> Result<(), RunError> {
    let playing = PLAYING.with(|playing| playing.try_borrow_mut().ok()?.take());
    let failure = playing.and_then(|playing| playing.machine.take_failure());
    match failure {
        Some(error) => {
            // The coroutines of a stopped game never go on.
            star_script_runtime::tasks::clear();
            Err(error)
        }
        None => Ok(()),
    }
}

/// Runs `hook` of `object` (a node or the game). After a runtime error,
/// here or in a coroutine, no more code runs and the game closes once the
/// frame ends.
pub(crate) fn run_hook(object: Value, hook: Hook) {
    let Some(playing) = PLAYING.with(|playing| playing.try_borrow().ok()?.clone()) else {
        return;
    };
    if !playing.machine.has_failed()
        && let Err(error) = playing.machine.run_hook(object, hook)
    {
        playing.machine.fail(error);
    }
    if playing.machine.has_failed() {
        playing.game.engine.quit();
    }
}
