use std::fs;

use super::*;
use crate::project::TestDirectory;

const MANIFEST: &str = "[project]\nname = \"Demo\"\nengine_version = \"0.1.0\"\ndimension = \"2d\"\nplatforms = [\"linux\"]\n\n[input.actions]\njump = [\"Space\"]\n";

const GAME: &str = "class Demo extends Game {\n  override func start() {\n    add_child(Rock())\n  }\n  override func update(delta: float) {\n    if Input.just_pressed(\"jump\") { quit() }\n  }\n}\n";

const ROCK: &str = "class Rock extends Node {\n  init() {\n    super.init(\"Rock\")\n  }\n}\n";

const SCENE: &str = "format = 1\n\n[[node]]\nname = \"Level\"\n\n[[node]]\nname = \"Big rock\"\nparent = 0\nclass = \"Rock\"\n";

/// A project in a test folder with `files` (paths inside the project).
fn project(test_name: &str, files: &[(&str, &str)]) -> TestDirectory {
    let directory = TestDirectory::new(test_name);
    fs::create_dir_all(directory.path().join(ASSETS_DIRECTORY)).expect("assets folder");
    for (path, text) in files {
        let path = directory.path().join(path);
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).expect("folder");
        }
        fs::write(path, text).expect("file");
    }
    directory
}

fn messages(check: &ProjectCheck) -> Vec<String> {
    check.problems.iter().map(ToString::to_string).collect()
}

#[test]
fn a_correct_game_of_several_files_has_no_problems() {
    let directory = project(
        "check_correct",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/enemies/rock.star", ROCK),
            ("assets/levels/one.scene", SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(check.scripts, 2);
    assert!(check.problems.is_empty(), "{:?}", messages(&check));
}

#[test]
fn script_mistakes_name_their_file_line_and_column() {
    let directory = project(
        "check_script_mistake",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            (
                "assets/enemies/rock.star",
                "class Rock extends Node {\n  var x = @\n}\n",
            ),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(
        messages(&check),
        ["assets/enemies/rock.star:2:11: unexpected character `@`"]
    );
}

#[test]
fn actions_missing_from_the_manifest_are_problems() {
    let manifest = MANIFEST.replace("jump", "fly");
    let directory = project(
        "check_unknown_action",
        &[
            (MANIFEST_FILE_NAME, &manifest),
            ("assets/demo.star", GAME),
            ("assets/rock.star", ROCK),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(check.problems.len(), 1, "{:?}", messages(&check));
    assert_eq!(check.problems[0].file, "assets/demo.star");
    assert!(check.problems[0].message.contains("jump"));
}

#[test]
fn scenes_that_name_unknown_classes_are_problems() {
    let directory = project(
        "check_scene_class",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            (
                "assets/demo.star",
                &GAME.replace("Rock()", "Node(\"Rock\")"),
            ),
            ("assets/one.scene", SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(
        messages(&check),
        ["assets/one.scene: node \"Big rock\": there is no class \"Rock\""]
    );
}

#[test]
fn projects_without_scripts_do_not_check_scene_classes() {
    let directory = project(
        "check_rust_game",
        &[(MANIFEST_FILE_NAME, MANIFEST), ("assets/one.scene", SCENE)],
    );

    let check = check_project(directory.path());

    assert_eq!(check.scripts, 0);
    assert!(check.problems.is_empty(), "{:?}", messages(&check));
}

#[test]
fn a_broken_manifest_is_a_problem_but_scripts_are_still_checked() {
    let directory = project(
        "check_broken_manifest",
        &[
            (MANIFEST_FILE_NAME, "[input.actions]\njump = 3\n"),
            ("assets/rock.star", "class Rock {\n  var x = @\n}\n"),
        ],
    );

    let check = check_project(directory.path());

    let files: Vec<&str> = check
        .problems
        .iter()
        .map(|problem| problem.file.as_str())
        .collect();
    assert_eq!(files, [MANIFEST_FILE_NAME, "assets/rock.star"]);
}

#[test]
fn programs_that_cannot_be_translated_are_problems() {
    let game = "class Demo extends Game {\n  init(level: int) {\n    super.init()\n  }\n}\n";
    let directory = project(
        "check_codegen",
        &[(MANIFEST_FILE_NAME, MANIFEST), ("assets/demo.star", game)],
    );

    let check = check_project(directory.path());

    assert_eq!(check.problems.len(), 1, "{:?}", messages(&check));
    assert!(
        check.problems[0]
            .to_string()
            .starts_with("assets/demo.star:")
    );
}

/// `Rock`, as a blueprint: `update` sets `speed` to the value of node 3.
fn rock_blueprint(node_3: &str) -> String {
    format!(
        "format = 1\nclass = \"Rock\"\nextends = \"Node\"\nbase_arguments = [\"Rock\"]\n\n\
         [[variable]]\nname = \"speed\"\ndefault = 2.0\n\n\
         [[graph]]\nname = \"update\"\noverride = true\n\
         parameters = [{{ name = \"delta\", type = \"float\" }}]\n\n\
         [[graph.node]]\nid = 1\nkind = \"entry\"\n\n\
         [[graph.node]]\nid = 2\nkind = \"set_variable\"\nvariable = \"speed\"\n\n\
         [[graph.node]]\nid = 3\n{node_3}\n\n\
         [[graph.link]]\nfrom = {{ node = 1, pin = \"then\" }}\nto = {{ node = 2, pin = \"in\" }}\n\n\
         [[graph.link]]\nfrom = {{ node = 3, pin = \"value\" }}\nto = {{ node = 2, pin = \"value\" }}\n"
    )
}

const DELTA_NODE: &str = "kind = \"get_variable\"\nvariable = \"speed\"";

#[test]
fn scripts_use_the_classes_of_blueprints() {
    let directory = project(
        "check_blueprint_class",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/rock.blueprint", &rock_blueprint(DELTA_NODE)),
            ("assets/one.scene", SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(check.scripts, 2);
    assert!(check.problems.is_empty(), "{:?}", messages(&check));
}

#[test]
fn compiler_mistakes_in_blueprints_name_their_node() {
    let text_node = "kind = \"literal\"\nvalue = \"fast\"";
    let directory = project(
        "check_blueprint_node",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/rock.blueprint", &rock_blueprint(text_node)),
        ],
    );

    let check = check_project(directory.path());

    let messages = messages(&check);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].starts_with("assets/rock.blueprint: graph `update`, node 3: "),
        "{messages:?}"
    );
}

#[test]
fn broken_blueprints_are_reported_alone() {
    let unwired_node = "kind = \"return\"";
    let directory = project(
        "check_blueprint_broken",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/rock.blueprint", &rock_blueprint(unwired_node)),
        ],
    );

    let check = check_project(directory.path());

    assert_ne!(check.problems, []);
    assert!(
        check
            .problems
            .iter()
            .all(|problem| problem.file == "assets/rock.blueprint"),
        "{:?}",
        messages(&check)
    );
}
