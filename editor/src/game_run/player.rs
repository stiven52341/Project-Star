//! The scene player: one program, built once, that plays the main scene of
//! any project without scripts, so “Play” does not build a game for them.

use std::io;
use std::path::{Path, PathBuf};

use super::{BUILD_DIRECTORY, ENGINE_CRATES, write_package};

/// The player's package, and its program. Its name cannot come from a
/// project: [`super::package_name`] gives none that starts with `_`.
pub(super) const PLAYER_PACKAGE: &str = "_star_scene_player";

/// Writes the player's package (only what changed) and gives its folder.
///
/// # Errors
///
/// Returns the error writing it, or the compiler's if it could not make
/// the player, which would be a mistake of the compiler itself.
pub(super) fn write_player() -> io::Result<PathBuf> {
    let program =
        star_script::player_program().map_err(|errors| io::Error::other(format!("{errors:?}")))?;
    let package = Path::new(BUILD_DIRECTORY).join(PLAYER_PACKAGE);
    write_package(PLAYER_PACKAGE, &program, Path::new(ENGINE_CRATES), &package)?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_project_gets_the_player_package_name() {
        assert_ne!(super::super::package_name(PLAYER_PACKAGE), PLAYER_PACKAGE);
    }

    #[test]
    fn the_player_package_is_written() {
        let package = write_player().expect("written");

        let code = package
            .join("src")
            .join("bin")
            .join(format!("{PLAYER_PACKAGE}.rs"));
        assert!(code.is_file(), "{}", code.display());
    }
}
