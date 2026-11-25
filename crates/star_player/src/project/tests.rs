use std::path::{Path, PathBuf};

use star_script::InputAction;
use star_script_interpreter::ProgramKind;

use super::*;

const MANIFEST: &str = "[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\"]\n";

const GAME: &str =
    "class Main extends Game {\n  override func start() {\n    add_child(Tally())\n  }\n}\n";

/// A blueprint with every kind of node, whose class is `Tally`.
const TALLY: &str = include_str!("../../../star_blueprint/tests/every_node.blueprint");

/// A project folder of a test, removed when the test ends.
struct TestProject(PathBuf);

impl TestProject {
    /// A project called `name` with `files` (paths inside the project) and
    /// an assets folder.
    fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let directory = std::env::temp_dir().join("star_player_tests").join(name);
        // Left over by an earlier run that stopped halfway.
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join(ASSETS_DIRECTORY)).expect("assets folder");
        for (path, text) in files {
            let path = directory.join(path);
            fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
            fs::write(path, text).expect("file");
        }
        Self(directory)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn problems(project: &TestProject) -> Vec<String> {
    match Project::read(project.path()).and_then(|read| read.program().map(|_| ())) {
        Ok(()) => Vec::new(),
        Err(problems) => problems.messages().to_vec(),
    }
}

#[test]
fn scripts_and_blueprints_make_one_program() {
    let folder = TestProject::new(
        "one_program",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/main.star", GAME),
            ("assets/blueprints/tally.blueprint", TALLY),
            ("assets/levels/one.scene", "not a scene the program reads"),
            ("assets/.draft.star", "a hidden file is left out"),
        ],
    );

    let project = Project::read(folder.path()).expect("a project");
    let program = project.program().expect("a program");

    assert_eq!(program.kind(), ProgramKind::Game);
    assert_eq!(program.title(), Some("Main"));
    assert_eq!(
        project.settings().actions,
        [InputAction {
            name: "jump".to_owned(),
            bindings: vec!["Space".to_owned()],
        }]
    );
    assert_eq!(project.assets_folder(), folder.path().join("assets"));
}

#[test]
fn a_main_scene_plays_without_scripts() {
    let manifest = "[project]\nname = \"Demo\"\nmain_scene = \"levels/one.scene\"\n";
    let folder = TestProject::new("scene_only", &[(MANIFEST_FILE_NAME, manifest)]);

    let program = Project::read(folder.path())
        .expect("a project")
        .program()
        .expect("a program");

    assert_eq!(program.kind(), ProgramKind::Game);
    assert_eq!(program.title(), Some("Demo"));
}

#[test]
fn script_mistakes_name_their_file_line_and_column() {
    let folder = TestProject::new(
        "script_mistake",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            (
                "assets/main.star",
                "class Main extends Game {\n  var lives: int = \"three\"\n}\n",
            ),
        ],
    );

    let problems = problems(&folder);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("assets/main.star:2:"),
        "{problems:?}"
    );
}

#[test]
fn blueprints_that_cannot_be_read_are_named() {
    let folder = TestProject::new(
        "broken_blueprint",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/main.star", GAME),
            ("assets/tally.blueprint", "format = \"one\"\n"),
        ],
    );

    let problems = problems(&folder);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("assets/tally.blueprint: "),
        "{problems:?}"
    );
}

#[test]
fn mistakes_in_the_code_of_a_blueprint_name_its_part() {
    // `Tally` already exists, so the class of the blueprint is declared twice.
    let folder = TestProject::new(
        "blueprint_mistake",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/main.star", &format!("{GAME}class Tally {{}}\n")),
            ("assets/tally.blueprint", TALLY),
        ],
    );

    let problems = problems(&folder);

    assert!(
        problems
            .iter()
            .any(|problem| problem.starts_with("assets/tally.blueprint: class: ")),
        "{problems:?}"
    );
}

#[test]
fn a_missing_manifest_is_a_problem() {
    let folder = TestProject::new("no_manifest", &[("assets/main.star", GAME)]);

    let problems = problems(&folder);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].starts_with("project.toml: "), "{problems:?}");
}

#[test]
fn a_project_without_scripts_or_main_scene_has_nothing_to_play() {
    let folder = TestProject::new("nothing", &[(MANIFEST_FILE_NAME, MANIFEST)]);

    assert_eq!(
        problems(&folder),
        [
            "project.toml: the project has nothing to play: no scripts, no blueprints and no main \
          scene"
        ]
    );
}

#[test]
fn what_the_interpreter_cannot_run_is_a_problem_too() {
    let folder = TestProject::new(
        "two_games",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/one.star", "class One extends Game {}\n"),
            ("assets/two.star", "class Two extends Game {}\n"),
        ],
    );

    let problems = problems(&folder);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("assets/two.star:1:"),
        "{problems:?}"
    );
}
