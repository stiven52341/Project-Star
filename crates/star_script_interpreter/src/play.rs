//! A game being played (decision 70). The runtime's `Host` functions,
//! such as running a node's `update`, take no interpreter, so the program
//! of the game is kept per thread while it plays, as the runtime keeps its
//! coroutines and input.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use star_script_runtime::nodes;

use crate::error::RunError;
use crate::host::{GameObject, Interpreted, NodeObject};
use crate::run::{Hook, Machine};
use crate::value::Value;

thread_local! {
    /// The game this thread plays, if any.
    static PLAYING: RefCell<Option<Playing>> = const { RefCell::new(None) };
    /// The root of the tree of the last game made on this thread, while
    /// that game lives: as compiled, it exists from the moment the game
    /// object does, before it is played.
    static GAME_ROOT: RefCell<Option<Weak<NodeObject>>> = const { RefCell::new(None) };
    /// The program loading a scene, whose classes the runtime's `Host`
    /// creates the scene's nodes as.
    static LOADING: RefCell<Option<Machine>> = const { RefCell::new(None) };
}

/// Runs `load` with `machine` as the program whose classes the nodes of
/// scenes are created as; a scene loaded by a node's `init` meanwhile
/// uses it too.
pub(crate) fn loading_scenes<R>(machine: &Machine, load: impl FnOnce() -> R) -> R {
    let previous = LOADING.with(|loading| {
        loading
            .try_borrow_mut()
            .ok()
            .and_then(|mut loading| loading.replace(machine.clone()))
    });
    let result = load();
    LOADING.with(|loading| {
        if let Ok(mut loading) = loading.try_borrow_mut() {
            *loading = previous;
        }
    });
    result
}

/// The program loading a scene, if one is.
pub(crate) fn scene_program() -> Option<Machine> {
    LOADING.with(|loading| loading.try_borrow().ok()?.clone())
}

/// Remembers `root` as the root of the game's tree.
pub(crate) fn set_game_root(root: &Rc<NodeObject>) {
    GAME_ROOT.with(|game_root| {
        if let Ok(mut game_root) = game_root.try_borrow_mut() {
            *game_root = Some(Rc::downgrade(root));
        }
    });
}

/// The root of the game's tree; `None` without a game (a console program).
pub(crate) fn game_root() -> Option<Rc<NodeObject>> {
    GAME_ROOT.with(|game_root| game_root.try_borrow().ok()?.as_ref()?.upgrade())
}

/// Whether `node` is in the game's tree, so its coroutines go on. Without
/// a game there is no tree to leave.
pub(crate) fn in_tree(node: &Rc<NodeObject>) -> bool {
    game_root().is_none_or(|root| nodes::is_same_or_ancestor::<Interpreted>(&root, node))
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

/// The program of the game this thread plays, if one plays.
pub(crate) fn machine() -> Option<Machine> {
    PLAYING.with(|playing| Some(playing.try_borrow().ok()?.as_ref()?.machine.clone()))
}

/// Gives the game this thread plays `machine`, its program reloaded.
pub(crate) fn replace_machine(machine: Machine) {
    PLAYING.with(|playing| {
        if let Ok(mut playing) = playing.try_borrow_mut()
            && let Some(playing) = playing.as_mut()
        {
            playing.machine = machine;
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
