//! Reloading a project's scripts into the game playing on this thread.

use std::fs;

use super::*;
use crate::project::{ASSETS_DIRECTORY, MANIFEST_FILE_NAME};

const MANIFEST: &str = "[project]\nname = \"Demo\"\n";

const GAME: &str = "class Main extends Game {\n  var frames = 0\n}\n";

/// A project folder with `main.star` written as `script`.
fn project(name: &str, script: &str) -> PathBuf {
    let directory = std::env::temp_dir().join("star_player_reload").join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(directory.join(ASSETS_DIRECTORY)).expect("assets folder");
    fs::write(directory.join(MANIFEST_FILE_NAME), MANIFEST).expect("manifest");
    write_script(&directory, script);
    directory
}

fn write_script(directory: &Path, script: &str) {
    fs::write(directory.join(ASSETS_DIRECTORY).join("main.star"), script).expect("script");
}

/// Starts playing the project in `directory` on this thread, without a
/// window.
fn start(directory: &Path) -> Rc<star_script_interpreter::host::GameObject> {
    let program = Project::read(directory)
        .and_then(|project| project.program())
        .expect("a project without mistakes");
    let output: Output = Rc::new(RefCell::new(std::io::sink()));
    program.new_game(&output).expect("a game")
}

#[test]
fn saved_scripts_reload_into_the_game_playing() {
    let directory = project("reloads", GAME);
    let _game = start(&directory);

    write_script(
        &directory,
        &GAME.replace("frames = 0", "frames = 0\n  var lives = 3"),
    );
    let message = reload_project(&directory);

    end_game().expect("a game that ends well");
    assert_eq!(message, "scripts reloaded: 1 objects kept");
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn programs_with_mistakes_are_reported_and_not_loaded() {
    let directory = project("mistake", GAME);
    let _game = start(&directory);

    write_script(
        &directory,
        "class Main extends Game {\n  var lives: int = \"three\"\n}\n",
    );
    let message = reload_project(&directory);

    end_game().expect("a game that ends well");
    assert!(message.starts_with("assets/main.star:2:"), "{message}");
    assert!(
        message.ends_with("the game goes on with the code it had"),
        "{message}"
    );
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn programs_the_objects_cannot_follow_are_not_loaded() {
    let directory = project(
        "refused",
        "class Helper {\n}\nclass Main extends Game {\n  var helper = Helper()\n}\n",
    );
    let _game = start(&directory);

    write_script(&directory, GAME);
    let message = reload_project(&directory);

    end_game().expect("a game that ends well");
    assert!(
        message.contains("the class `Helper` was removed"),
        "{message}"
    );
    assert!(message.contains("play again"), "{message}");
    let _ = fs::remove_dir_all(&directory);
}
