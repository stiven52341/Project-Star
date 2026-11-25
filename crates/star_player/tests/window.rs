//! Small project folders played by the player in a real window of the
//! engine, as “Play” plays them: a game with a blueprint, a project with
//! only a main scene, and projects that must stop saying why.
//!
//! It needs a screen and a GPU (or a software renderer), and the window
//! needs the main thread, so it has no test harness and only plays when
//! asked: `cargo test -p star_player --test window -- --ignored`. The
//! engine's window can be opened once per process, so the test plays each
//! project in a child process of itself (`window --play <project>`), one
//! after another.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// How a project must end.
enum Ending {
    /// It ends well, having printed this.
    Prints(&'static str),
    /// It stops with an error that says this.
    Fails(&'static str),
}

/// A project folder of the test.
struct TestProject {
    name: &'static str,
    /// Its files, by their paths inside the project.
    files: &'static [(&'static str, &'static str)],
    ending: Ending,
}

const MANIFEST: &str =
    "[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\", \"GamepadSouth\"]\n";

/// A blueprint with every kind of node: a `Tally` prints whether the
/// frames it saw are odd or even.
const TALLY: &str = include_str!("../../star_blueprint/tests/every_node.blueprint");

const BLUEPRINT_GAME: &str = "class Main extends Game {\n  var frames = 0\n  \
     override func start() {\n    add_child(Tally())\n  }\n  \
     override func update(delta: float) {\n    frames += 1\n    \
     Debug.print(\"frame \" + frames.to_string())\n    \
     if frames == 3 {\n      quit()\n    }\n  }\n}\n";

const PROJECTS: [TestProject; 5] = [
    TestProject {
        name: "blueprint_game",
        files: &[
            ("project.toml", MANIFEST),
            ("assets/main.star", BLUEPRINT_GAME),
            ("assets/blueprints/tally.blueprint", TALLY),
        ],
        ending: Ending::Prints("frame 1\nodd\nframe 2\neven\nframe 3\nodd\n"),
    },
    // Never quits by itself: played only with a scene it cannot load.
    TestProject {
        name: "scene_only",
        files: &[(
            "project.toml",
            "[project]\nname = \"Scene only\"\nmain_scene = \"levels/nowhere.scene\"\n",
        )],
        ending: Ending::Fails("the main scene, \"levels/nowhere.scene\", could not be loaded"),
    },
    TestProject {
        name: "mistake",
        files: &[
            ("project.toml", MANIFEST),
            (
                "assets/main.star",
                "class Main extends Game {\n  var lives: int = \"three\"\n}\n",
            ),
        ],
        ending: Ending::Fails("assets/main.star:2:"),
    },
    TestProject {
        name: "misspelled_binding",
        files: &[
            (
                "project.toml",
                "[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Spcae\"]\n",
            ),
            ("assets/main.star", BLUEPRINT_GAME),
            ("assets/blueprints/tally.blueprint", TALLY),
        ],
        ending: Ending::Fails("unknown input binding \"Spcae\""),
    },
    TestProject {
        name: "fault_at_creation",
        files: &[
            ("project.toml", MANIFEST),
            (
                "assets/main.star",
                "class Main extends Game {\n  var lives = [1, 2][3]\n}\n",
            ),
        ],
        ending: Ending::Fails("StarScript runtime error: index 3 is out of range"),
    },
];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(position) = arguments.iter().position(|argument| argument == "--play") {
        let name = arguments.get(position + 1).expect("the name of a project");
        let project = PROJECTS
            .iter()
            .find(|project| project.name == name)
            .expect("a project of the test");
        return match star_player::play(&folder(project)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        };
    }
    if !arguments.iter().any(|argument| argument == "--ignored") {
        println!("window: ignored (pass `-- --ignored` to open a window)");
        return ExitCode::SUCCESS;
    }
    let failures: Vec<String> = PROJECTS
        .iter()
        .filter_map(|project| check(project).err())
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
    println!("window: ok ({} projects)", PROJECTS.len());
    ExitCode::SUCCESS
}

/// Writes `project` and plays it in a child process, checking how it ended.
fn check(project: &TestProject) -> Result<(), String> {
    write(project);
    let executable = std::env::current_exe().expect("the test's own executable");
    let output = Command::new(executable)
        .args(["--play", project.name])
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let name = project.name;
    match (&project.ending, output.status.success()) {
        (Ending::Prints(expected), true) if stdout == *expected => Ok(()),
        (Ending::Prints(expected), true) => Err(format!(
            "{name}: unexpected output\n--- expected\n{expected}--- found\n{stdout}"
        )),
        (Ending::Prints(_), false) => Err(format!("{name} failed:\n{stderr}")),
        (Ending::Fails(reason), false) if stderr.contains(reason) => Ok(()),
        (Ending::Fails(reason), _) => Err(format!(
            "{name}: should have stopped saying `{reason}`:\n{stderr}"
        )),
    }
}

/// The folder of `project`.
fn folder(project: &TestProject) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("player_projects")
        .join(project.name)
}

/// Writes the files of `project` in its folder, with an assets folder.
fn write(project: &TestProject) {
    let folder = folder(project);
    // Files of an earlier version of the test would be read too.
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("assets")).expect("assets folder");
    for (path, text) in project.files {
        let path = folder.join(path);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
        std::fs::write(path, text).expect("file written");
    }
}
