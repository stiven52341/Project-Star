//! The arguments of the scene player, the program that plays the main
//! scene of any project without scripts (decision 63).

use star_engine::star_core::{EngineError, EngineResult};

/// Where the scene to play is among the player's arguments.
pub const SCENE_ARGUMENT: usize = 1;
/// Where the title of the window is among the player's arguments.
pub const TITLE_ARGUMENT: usize = 2;

/// The player's argument at `index`, which is the `what` of the player.
///
/// # Errors
///
/// [`EngineError::Initialization`] telling how to play a scene, if the
/// argument is missing.
pub fn argument(index: usize, what: &str) -> EngineResult<String> {
    argument_of(std::env::args(), index, what)
}

/// The argument at `index` of `arguments`, as [`argument`].
fn argument_of(
    mut arguments: impl Iterator<Item = String>,
    index: usize,
    what: &str,
) -> EngineResult<String> {
    arguments
        .nth(index)
        .ok_or_else(|| EngineError::Initialization {
            subsystem: "scene player",
            reason: format!("the {what} is missing: play a scene with `player <scene> <title>`"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn the_scene_and_the_title_are_read_in_order() {
        let given = ["player", "scenes/main.scene", "Demo"];

        let scene = argument_of(arguments(&given), SCENE_ARGUMENT, "scene");
        let title = argument_of(arguments(&given), TITLE_ARGUMENT, "title");

        assert_eq!(scene.ok().as_deref(), Some("scenes/main.scene"));
        assert_eq!(title.ok().as_deref(), Some("Demo"));
    }

    #[test]
    fn a_missing_argument_says_how_to_play() {
        let error =
            argument_of(arguments(&["player"]), TITLE_ARGUMENT, "title").expect_err("missing");

        assert!(
            error.to_string().contains("the title is missing"),
            "{error}"
        );
        assert!(error.to_string().contains("`player <scene> <title>`"));
    }
}
