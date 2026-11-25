//! Hot reload (decision 76): a game played without a window gets a new
//! version of its program between two frames.

use star_script::check_source;
use star_script_runtime::nodes::Host as _;
use star_script_runtime::{game, tasks};

use super::{DELTA, buffer};
use crate::host::Interpreted;
use crate::{Program, ReloadError, Reloaded, end_game, reload};

fn lower(source: &str) -> Program {
    let checked = check_source(source).expect("a valid program");
    Program::lower(&checked).expect("a program the interpreter runs")
}

/// Plays `first` for `before` frames, reloads it as `second` and plays
/// `after` frames more: what it printed, and what the reload did.
fn play_reloading(
    first: &str,
    second: &str,
    before: usize,
    after: usize,
) -> (String, Result<Reloaded, ReloadError>) {
    let (buffer, output) = buffer();
    let game = lower(first)
        .new_game(&output)
        .expect("a game that can be created");
    let second = lower(second);
    Interpreted::start_game(&game);
    let mut nodes = Vec::new();
    let mut reloaded = None;
    for frame in 0..before + after {
        if frame == before {
            reloaded = Some(reload(&second));
        }
        tasks::frame_started(DELTA);
        game::run_frame::<Interpreted>(&game, DELTA, &mut nodes);
        tasks::resume();
    }
    end_game().expect("a game that ends well");
    let printed = String::from_utf8(buffer.take()).expect("UTF-8 output");
    (printed, reloaded.expect("a reload"))
}

const COUNTER: &str = "class Main extends Game {\n var frames = 0\n \
                       override func update(delta: float) {\n frames += 1\n \
                       Debug.print(\"old \" + frames.to_string())\n }\n}\n";

#[test]
fn changed_methods_take_effect_and_fields_keep_their_values() {
    let second = COUNTER.replace("old ", "new ");

    let (printed, reloaded) = play_reloading(COUNTER, &second, 2, 2);

    assert_eq!(printed, "old 1\nold 2\nnew 3\nnew 4\n");
    assert_eq!(
        reloaded.expect("a reload"),
        Reloaded {
            objects: 1,
            coroutines_ended: 0
        }
    );
}

#[test]
fn fields_are_kept_started_reset_or_dropped_by_name_and_type() {
    let first = "class Stats {\n var hits = 0\n}\n\
                 class Mover extends Node {\n var steps = 0\n init() {\n super.init(\"Mover\")\n }\n \
                 override func update(delta: float) {\n steps += 1\n }\n}\n\
                 class Main extends Game {\n var stats = Stats()\n var speed = 5\n var gone = \"x\"\n \
                 init() {\n add_child(Mover())\n }\n \
                 override func update(delta: float) {\n stats.hits += 1\n speed += 1\n }\n}\n";
    let second = "class Stats {\n var misses = 7\n var hits = 0\n}\n\
                  class Mover extends Node {\n var steps = 0\n var path = [1, 2]\n init() {\n super.init(\"Mover\")\n }\n \
                  override func update(delta: float) {\n steps += 1\n \
                  Debug.print(\"mover \" + steps.to_string() + \" \" + path.length().to_string())\n }\n}\n\
                  class Main extends Game {\n var stats = Stats()\n var speed = 1.5\n var label = \"new\"\n \
                  override func update(delta: float) {\n stats.hits += 1\n speed += 1.0\n \
                  Debug.print(stats.hits.to_string() + \" \" + stats.misses.to_string() + \" \" + speed.to_string() + \" \" + label)\n }\n}\n";

    let (printed, reloaded) = play_reloading(first, second, 2, 1);

    // `speed` was an `int` and is a `float` now: it starts again.
    assert_eq!(printed, "3 7 2.5 new\nmover 3 2\n");
    assert_eq!(reloaded.expect("a reload").objects, 3);
}

#[test]
fn a_removed_class_of_a_living_object_keeps_the_old_code() {
    let first = "class Mover extends Node {\n init() {\n super.init(\"Mover\")\n }\n}\n\
                 class Main extends Game {\n var frames = 0\n init() {\n add_child(Mover())\n }\n \
                 override func update(delta: float) {\n frames += 1\n Debug.print(\"old \" + frames.to_string())\n }\n}\n";
    let second = COUNTER.replace("old ", "new ");

    let (printed, reloaded) = play_reloading(first, &second, 1, 1);

    assert_eq!(printed, "old 1\nold 2\n");
    assert!(
        matches!(&reloaded, Err(ReloadError::ClassRemoved(class)) if class == "Mover"),
        "{reloaded:?}"
    );
}

#[test]
fn a_class_whose_engine_class_changed_keeps_the_old_code() {
    let first = "class Helper {\n}\n\
                 class Main extends Game {\n var helper = Helper()\n}\n";
    let second = "class Helper extends Node {\n init() {\n super.init(\"Helper\")\n }\n}\n\
                  class Main extends Game {\n var helper = Helper()\n}\n";

    let (_, reloaded) = play_reloading(first, second, 1, 1);

    assert!(
        matches!(&reloaded, Err(ReloadError::EngineClassChanged(class)) if class == "Helper"),
        "{reloaded:?}"
    );
}

#[test]
fn a_new_field_only_init_gives_a_value_keeps_the_old_code() {
    let second = "class Main extends Game {\n var frames = 0\n var target: Node\n \
                  init() {\n target = Node(\"target\")\n }\n}\n";

    let (printed, reloaded) = play_reloading(COUNTER, second, 1, 1);

    assert_eq!(printed, "old 1\nold 2\n");
    assert!(
        matches!(&reloaded, Err(ReloadError::FieldNeedsInit { class, field })
            if class == "Main" && field == "target"),
        "{reloaded:?}"
    );
}

#[test]
fn coroutines_started_before_end_and_are_counted() {
    let first = "class Main extends Game {\n override func start() {\n tick()\n }\n \
                 async func tick() {\n while true {\n Debug.print(\"tick\")\n await Time.next_frame()\n }\n }\n}\n";

    let (printed, reloaded) = play_reloading(first, first, 2, 2);

    // None after the reload.
    assert_eq!(printed, "tick\ntick\n");
    assert_eq!(reloaded.expect("a reload").coroutines_ended, 1);
}

#[test]
fn dropped_objects_are_not_remembered() {
    let first = "class Stats {\n}\n\
                 class Main extends Game {\n init() {\n for i in 0..200 {\n var stats = Stats()\n }\n }\n}\n";

    let (_, reloaded) = play_reloading(first, first, 1, 1);

    assert_eq!(reloaded.expect("a reload").objects, 1);
}

#[test]
fn only_a_game_playing_is_reloaded() {
    let game = lower(COUNTER);
    let console = lower("class Main {\n static func main() {\n }\n}\n");

    assert!(matches!(reload(&game), Err(ReloadError::NotPlaying)));
    let (_, output) = buffer();
    let _game = game.new_game(&output).expect("a game");
    assert!(matches!(reload(&console), Err(ReloadError::NotAGame)));
    end_game().expect("a game that ends well");
}
