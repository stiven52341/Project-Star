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

/// The player's build script. On Windows a program's main thread gets a
/// stack of 1 MiB, where Linux and macOS give 8 MiB; the player asks for
/// 8 MiB there too, so the nested calls StarScript allows fit in it
/// interpreted (decision 77), and a call that never ends stops with its
/// runtime error rather than a stack overflow. It only changes the
/// player's own link, so the engine's build is not repeated.
const BUILD_SCRIPT: &str = "//! Gives the player's main thread the stack of Linux and macOS on\n\
                            //! Windows (written by the editor).\n\n\
                            /// 8 MiB, what the main thread gets on Linux and macOS.\n\
                            const STACK_BYTES: u32 = 8 * 1024 * 1024;\n\n\
                            fn main() {\n    \
                            let variable = |name| std::env::var(name).unwrap_or_default();\n    \
                            if variable(\"CARGO_CFG_TARGET_OS\") != \"windows\" {\n        \
                            return;\n    }\n    \
                            let argument = if variable(\"CARGO_CFG_TARGET_ENV\") == \"msvc\" {\n        \
                            format!(\"/STACK:{STACK_BYTES}\")\n    \
                            } else {\n        \
                            format!(\"-Wl,--stack,{STACK_BYTES}\")\n    };\n    \
                            println!(\"cargo:rustc-link-arg-bins={argument}\");\n}\n";

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
    write_if_changed(&package.join("build.rs"), BUILD_SCRIPT)?;
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
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::*;

    /// Tests that write the player's package take turns: one could read a
    /// file while another is writing it.
    static PACKAGE: Mutex<()> = Mutex::new(());

    fn write_player_alone() -> (PathBuf, MutexGuard<'static, ()>) {
        let turn = PACKAGE.lock().unwrap_or_else(PoisonError::into_inner);
        (write_player().expect("written"), turn)
    }

    #[test]
    fn the_player_package_is_written_with_the_engine_lockfile() {
        let (package, _turn) = write_player_alone();

        let main = fs::read_to_string(package.join("src").join("main.rs")).expect("main");
        assert!(main.contains("star_player::main()"), "{main}");
        let written = fs::read_to_string(package.join("Cargo.lock")).expect("lockfile");
        let engine = fs::read_to_string(ENGINE_LOCKFILE).expect("engine lockfile");
        assert_eq!(written, engine);
    }

    #[test]
    fn the_player_asks_windows_for_the_stack_of_linux_and_macos() {
        let (package, _turn) = write_player_alone();

        let script = fs::read_to_string(package.join("build.rs")).expect("build script");
        assert!(script.contains("8 * 1024 * 1024"), "{script}");
        assert!(script.contains("\"/STACK:{STACK_BYTES}\""), "{script}");
        assert!(script.contains("\"-Wl,--stack,{STACK_BYTES}\""), "{script}");
        assert!(script.contains("cargo:rustc-link-arg-bins="), "{script}");
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
