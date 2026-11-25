//! An interpreted StarScript game played in a real window of the engine,
//! through `star_script_engine` as a compiled game is (decision 70).
//!
//! It needs a screen and a GPU (or a software renderer), and the window
//! needs the main thread, so it has no test harness and only plays when
//! asked: `cargo test -p star_script_interpreter --test window -- --ignored`.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;

use star_engine::app::App;
use star_engine::star_core::EngineError;
use star_script::{GameSettings, InputAction, check_source_with};
use star_script_interpreter::host::Interpreted;
use star_script_interpreter::{Output, Program, end_game};

/// Counts its frames, with a camera of its own that moves, and closes
/// itself after the third frame.
const GAME: &str = "
class Eye extends Camera2D {
    init() {
        super.init(\"Eye\")
        main = true
    }

    override func start() {
        Debug.print(\"eye start\")
    }

    override func update(delta: float) {
        x += 1.0
    }
}

class CountingGame extends Game {
    var frames = 0

    init() {
        add_child(Eye())
    }

    override func start() {
        background = Color(0.2, 0.3, 0.4, 1.0)
        Debug.print(\"start\")
    }

    override func update(delta: float) {
        frames += 1
        Debug.print(\"frame \" + frames.to_string() + \" \" + (delta >= 0.0).to_string())
        if Input.is_down(\"jump\") || Input.has_pointer() && Input.pointer_x() > 1000000.0 {
            Debug.print(\"unexpected input\")
        }
        if frames == 3 {
            Debug.print(\"camera x \" + (camera?.x ?? 0.0).to_string())
            quit()
        }
    }
}
";

/// What the game prints: the camera has moved in the two frames after the
/// one that started it.
const EXPECTED: &str = "start\nframe 1 true\neye start\nframe 2 true\nframe 3 true\ncamera x 2.0\n";

fn main() {
    if !std::env::args().any(|argument| argument == "--ignored") {
        println!("window: ignored (pass `-- --ignored` to open a window)");
        return;
    }
    let settings = GameSettings {
        actions: vec![InputAction {
            name: "jump".to_owned(),
            bindings: vec!["Space".to_owned()],
        }],
        main_scene: None,
    };
    let checked = check_source_with(GAME, &settings).expect("a valid game");
    let program = Program::lower(&checked).expect("a game the interpreter runs");
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    let actions = ["jump"];
    star_script_runtime::input::declare_actions(&actions);
    let app = App::new().with_title("Interpreted game");
    star_script_engine::game::run::<Interpreted>(app, &actions, || {
        program
            .new_game(&output)
            .map_err(|error| EngineError::Initialization {
                subsystem: "interpreter",
                reason: error.to_string(),
            })
    })
    .expect("the game plays in a window");
    end_game().expect("a game that ends without errors");
    let printed = String::from_utf8(buffer.take()).expect("UTF-8 output");
    assert_eq!(printed, EXPECTED);
    println!("window: ok");
}
