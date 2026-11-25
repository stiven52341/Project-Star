//! The player's package: one program, built once, whose `main` is
//! `star_player::main`, which plays the project in the folder it runs in
//! with its StarScript interpreted (decision 74). It is built outside the
//! editor's workspace, whose features differ from a game's (decision 63).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{BUILD_DIRECTORY, ENGINE_CRATES, ENGINE_LOCKFILE};

/// The player's package, and its program.
pub(super) const PLAYER_PACKAGE: &str = "_star_player";
/// The crate that plays projects, among the engine's.
const PLAYER_CRATE: &str = "star_player";
/// The player's whole code.
const MAIN: &str = "//! Plays the project in the current folder (written by the editor).\n\n\
                    fn main() -> std::process::ExitCode {\n    star_player::main()\n}\n";

/// Writes the player's package (only what changed) and gives its folder.
///
/// # Errors
///
/// Returns the error reading the engine's lockfile or writing the files.
pub(super) fn write_player() -> io::Result<PathBuf> {
    let package = Path::new(BUILD_DIRECTORY).join(PLAYER_PACKAGE);
    fs::create_dir_all(package.join("src"))?;
    write_if_changed(
        &package.join("Cargo.toml"),
        &manifest(Path::new(ENGINE_CRATES)),
    )?;
    // The same versions as the engine and the editor, so the engine's build
    // can be shared.
    write_if_changed(
        &package.join("Cargo.lock"),
        &fs::read_to_string(ENGINE_LOCKFILE)?,
    )?;
    write_if_changed(&package.join("src").join("main.rs"), MAIN)?;
    Ok(package)
}

/// The player's `Cargo.toml`, with the player crate in `engine_crates`.
/// Its dependencies are optimized as in ProjectStar, so games stay
/// playable in a build without optimizations.
fn manifest(engine_crates: &Path) -> String {
    let player = engine_crates
        .join(PLAYER_CRATE)
        .to_string_lossy()
        .into_owned();
    let path = player.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "# Written by the ProjectStar editor.\n\
         [package]\n\
         name = \"{PLAYER_PACKAGE}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         \n\
         [dependencies]\n\
         {PLAYER_CRATE} = {{ path = \"{path}\" }}\n\
         \n\
         # Its own workspace, inside the editor's `target` folder.\n\
         [workspace]\n\
         \n\
         [profile.dev]\n\
         opt-level = 1\n\
         \n\
         [profile.dev.package.\"*\"]\n\
         opt-level = 3\n\
         debug = \"line-tables-only\"\n"
    )
}

/// Writes `text` to `path` unless it already holds exactly that: Cargo
/// rebuilds a package whose files were written again, even if unchanged.
fn write_if_changed(path: &Path, text: &str) -> io::Result<()> {
    match fs::read_to_string(path) {
        Ok(current) if current == text => Ok(()),
        Ok(_) => fs::write(path, text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::write(path, text),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_player_package_is_written_with_the_engine_lockfile() {
        let package = write_player().expect("written");

        let main = fs::read_to_string(package.join("src").join("main.rs")).expect("main");
        assert!(main.contains("star_player::main()"), "{main}");
        let written = fs::read_to_string(package.join("Cargo.lock")).expect("lockfile");
        let engine = fs::read_to_string(ENGINE_LOCKFILE).expect("engine lockfile");
        assert_eq!(written, engine);
    }

    #[test]
    fn no_project_gets_the_player_package_name() {
        assert_ne!(
            super::super::compiled::package_name(PLAYER_PACKAGE),
            PLAYER_PACKAGE
        );
    }

    #[test]
    fn the_player_depends_on_the_player_crate() {
        let manifest = manifest(Path::new("/engine"));

        assert!(manifest.contains("star_player = { path = \""), "{manifest}");
        assert!(manifest.contains("star_player\" }"), "{manifest}");
        assert!(manifest.contains("name = \"_star_player\""));
    }
}
