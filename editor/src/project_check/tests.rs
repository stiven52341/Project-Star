use std::fs;

use star_blueprint::{NodeId, Origin};

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

fn check_project(directory: &std::path::Path) -> ProjectCheck {
    check_project_with(directory, &OpenTexts::new(), CheckGoal::Compile)
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
    let program = check.program.expect("the game's Rust");
    assert!(program.uses_engine());
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
                "class Rock extends Node {\n  var x = $\n}\n",
            ),
        ],
    );

    let check = check_project(directory.path());

    assert_eq!(
        messages(&check),
        ["assets/enemies/rock.star:2:11: unexpected character `$`"]
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
    assert_eq!(check.program, None, "a game with problems is not played");
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
    assert_eq!(check.program, None);
}

#[test]
fn a_broken_manifest_is_a_problem_but_scripts_are_still_checked() {
    let directory = project(
        "check_broken_manifest",
        &[
            (MANIFEST_FILE_NAME, "[input.actions]\njump = 3\n"),
            ("assets/rock.star", "class Rock {\n  var x = $\n}\n"),
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

    for goal in [CheckGoal::Compile, CheckGoal::Writing] {
        let check = check_project_with(directory.path(), &OpenTexts::new(), goal);

        assert_eq!(check.problems.len(), 1, "{:?}", messages(&check));
        assert!(
            check.problems[0]
                .to_string()
                .starts_with("assets/demo.star:")
        );
    }
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
    assert_eq!(
        check.problems[0].origin,
        Some(Origin::Node {
            graph: "update".to_owned(),
            node: NodeId(3),
        })
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
    assert!(
        check
            .problems
            .iter()
            .any(|problem| matches!(problem.origin, Some(Origin::Node { .. }))),
        "{:?}",
        messages(&check)
    );
}

#[test]
fn open_texts_are_checked_instead_of_their_files() {
    let directory = project(
        "check_open_texts",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/rock.star", ROCK),
        ],
    );
    let open = OpenTexts::from([(
        "rock.star".to_owned(),
        "class Rock {\n  var x = $\n}\n".to_owned(),
    )]);

    let check = check_project_with(directory.path(), &open, CheckGoal::Writing);

    assert_eq!(
        messages(&check),
        ["assets/rock.star:2:11: unexpected character `$`"]
    );
}

#[test]
fn the_outline_knows_the_locals_of_each_script() {
    let directory = project(
        "check_locals",
        &[
            (MANIFEST_FILE_NAME, MANIFEST),
            ("assets/demo.star", GAME),
            ("assets/enemies/rock.star", ROCK),
        ],
    );

    let check = check_project_with(directory.path(), &OpenTexts::new(), CheckGoal::Writing);

    let outline = check.outline.expect("the program was read");
    let delta = GAME.find("delta").expect("a parameter");
    let local = outline
        .locals_of("demo.star")
        .and_then(|locals| locals.get(&delta))
        .expect("the parameter is known");
    assert_eq!(local.name, "delta");
    assert_eq!(outline.classes.type_name(&local.ty), "float");
}

/// `project.toml` of a project whose main scene is `scenes/main.scene`.
const MAIN_SCENE_MANIFEST: &str = "[project]\nname = \"Demo\"\nengine_version = \"0.1.0\"\ndimension = \"2d\"\nplatforms = [\"linux\"]\nmain_scene = \"scenes/main.scene\"\n";

const SEEN_SCENE: &str = "format = 1\n[[node]]\nname = \"Level\"\n[[node]]\nname = \"Eye\"\nparent = 0\n[node.camera]\nmain = true\n";

#[test]
fn projects_without_scripts_play_their_main_scene() {
    let directory = project(
        "check_main_scene",
        &[
            (MANIFEST_FILE_NAME, MAIN_SCENE_MANIFEST),
            ("assets/scenes/main.scene", SEEN_SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert!(check.problems.is_empty(), "{:?}", messages(&check));
    let program = check.program.expect("the game's Rust");
    assert_eq!(program.entry_point, EntryPoint::MainScene);
    let scene = check.scene_only.expect("played by the scene player");
    assert_eq!(scene.path, "scenes/main.scene");
    assert_eq!(scene.title, "Demo");
}

#[test]
fn main_scenes_with_scripts_are_built_as_games() {
    let directory = project(
        "check_main_scene_scripts",
        &[
            (MANIFEST_FILE_NAME, MAIN_SCENE_MANIFEST),
            ("assets/enemies/rock.star", ROCK),
            ("assets/scenes/main.scene", SEEN_SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert!(check.problems.is_empty(), "{:?}", messages(&check));
    assert_eq!(
        check.program.map(|program| program.entry_point),
        Some(EntryPoint::MainScene)
    );
    assert!(check.scene_only.is_none());
}

#[test]
fn main_scenes_need_a_main_camera_and_to_exist() {
    let unseen = project(
        "check_main_scene_unseen",
        &[
            (MANIFEST_FILE_NAME, MAIN_SCENE_MANIFEST),
            ("assets/scenes/main.scene", SCENE),
        ],
    );
    let missing = project(
        "check_main_scene_missing",
        &[(MANIFEST_FILE_NAME, MAIN_SCENE_MANIFEST)],
    );

    let unseen = check_project(unseen.path());
    let missing = check_project(missing.path());

    assert!(unseen.program.is_none());
    assert!(unseen.scene_only.is_none());
    assert_eq!(unseen.problems.len(), 2, "{:?}", messages(&unseen));
    assert!(
        unseen.problems[0]
            .to_string()
            .contains("assets/scenes/main.scene"),
        "{:?}",
        messages(&unseen)
    );
    assert!(missing.program.is_none());
    assert_eq!(missing.problems.len(), 1, "{:?}", messages(&missing));
}

#[test]
fn games_ignore_the_main_camera_of_the_main_scene() {
    let directory = project(
        "check_main_scene_game",
        &[
            (
                MANIFEST_FILE_NAME,
                MAIN_SCENE_MANIFEST
                    .replace(
                        "[project]",
                        "[input.actions]\njump = [\"Space\"]\n\n[project]",
                    )
                    .as_str(),
            ),
            ("assets/demo.star", GAME),
            ("assets/enemies/rock.star", ROCK),
            ("assets/scenes/main.scene", SCENE),
        ],
    );

    let check = check_project(directory.path());

    assert!(check.problems.is_empty(), "{:?}", messages(&check));
    assert!(check.scene_only.is_none());
    assert_eq!(check.program.unwrap().entry_point, EntryPoint::Game);
}
