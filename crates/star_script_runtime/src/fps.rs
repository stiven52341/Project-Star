//! StarScript's `Debug.show_fps`: the game asks for the engine's
//! frames-per-second counter to be shown or hidden (decision 85). This
//! crate cannot reach the engine, so the wish is kept until the game's
//! loop passes it on; console programs never do, and nothing happens. The
//! game runs on one thread, so it is kept per thread.

use std::cell::Cell;

thread_local! {
    static REQUEST: Cell<Option<bool>> = const { Cell::new(None) };
}

/// `Debug.show_fps(shown)`: shows the counter, or hides it, from the end
/// of the frame. The last wish of a frame wins.
pub fn show_fps(shown: bool) {
    REQUEST.with(|request| request.set(Some(shown)));
}

/// What the game asked of the counter since the last call, if anything;
/// the wish is then forgotten.
#[must_use]
pub fn take_request() -> Option<bool> {
    REQUEST.with(Cell::take)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_wish_is_taken_once() {
        show_fps(true);
        show_fps(false);

        assert_eq!(take_request(), Some(false));
        assert_eq!(take_request(), None);
    }
}
