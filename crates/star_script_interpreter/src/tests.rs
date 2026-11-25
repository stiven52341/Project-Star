//! Lowering and running small programs.

use std::cell::RefCell;
use std::rc::Rc;

use star_script::check_source;

use star_script_runtime::nodes::Host as _;
use star_script_runtime::{game, tasks};

use crate::host::Interpreted;
use crate::{LowerErrorKind, Output, Program, ProgramKind, RunError, end_game};

/// What `source` prints when interpreted, or why it stopped.
fn run(source: &str) -> Result<String, RunError> {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let (buffer, output) = buffer();
    program.run(&output)?;
    Ok(String::from_utf8(buffer.take()).expect("UTF-8 output"))
}

/// Why `source` cannot be prepared.
fn lowering_error(source: &str) -> LowerErrorKind {
    let checked = check_source(source).expect("a valid program");
    Program::lower(&checked)
        .expect_err("a program the interpreter refuses")
        .kind
}

#[test]
fn main_prints_lines() {
    let source = "class Main {\n static func main() {\n Debug.print(\"Hola\")\n \
                  Debug.print(twice(21).to_string())\n }\n \
                  static func twice(value: int) -> int {\n return value * 2\n }\n}\n";
    assert_eq!(run(source).ok().as_deref(), Some("Hola\n42\n"));
}

#[test]
fn loops_leave_and_skip() {
    let source = "class Main {\n static func main() {\n var total = 0\n \
                  for i in 0..10 {\n if i == 2 { continue }\n if i == 5 { break }\n \
                  total += i\n }\n while total < 100 { total *= 3 }\n \
                  Debug.print(total.to_string())\n }\n}\n";
    // 0 + 1 + 3 + 4 = 8, then 24, 72, 216.
    assert_eq!(run(source).ok().as_deref(), Some("216\n"));
}

#[test]
fn runtime_errors_stop_the_program() {
    let source = "class Main {\n static func main() {\n var zero = 0\n \
                  Debug.print(\"before\")\n Debug.print((1 % zero).to_string())\n }\n}\n";
    let error = run(source).expect_err("a runtime error");
    assert_eq!(
        error.to_string(),
        "StarScript runtime error: integer remainder by zero"
    );
}

#[test]
fn programs_need_one_main() {
    let source = "class Tools {\n static func helper() {\n }\n}\n";
    assert_eq!(lowering_error(source), LowerErrorKind::NoEntryPoint);
}

#[test]
fn what_cannot_run_yet_is_refused_before_running() {
    let sprite = "class Player extends Sprite {\n init() {\n super.init(\"player\", \"hero.png\")\n }\n}\nclass Main {\n static func main() {\n }\n}\n";
    assert_eq!(
        lowering_error(sprite),
        LowerErrorKind::Unsupported("classes that extend this engine class")
    );
    let method =
        "class Main {\n static func main() {\n Node(\"a\").add_child(Node(\"b\"))\n }\n}\n";
    assert_eq!(
        lowering_error(method),
        LowerErrorKind::Unsupported("this method of the engine")
    );
    let tags = "class Main {\n static func main() {\n Debug.print(Node(\"a\").tags.length().to_string())\n }\n}\n";
    assert_eq!(
        lowering_error(tags),
        LowerErrorKind::Unsupported("this field of the engine")
    );
}

#[test]
fn subclasses_override_and_call_their_base() {
    // The subclass is written first: tables are built bases first.
    let source = "class Loud extends Quiet {\n var volume: int\n \
                  init(level: int) {\n super.init(\"loud\")\n volume = level\n }\n \
                  override func say() -> string {\n return super.say() + \"!\" + volume.to_string()\n }\n}\n\
                  class Quiet {\n var word: string\n init(word: string) {\n self.word = word\n }\n \
                  func say() -> string {\n return word\n }\n}\n\
                  class Main {\n static func main() {\n const voice: Quiet = Loud(3)\n \
                  Debug.print(voice.say())\n Debug.print((voice is Loud).to_string())\n \
                  const quiet = Quiet(\"shh\")\n Debug.print(((quiet as Loud) == null).to_string())\n }\n}\n";
    assert_eq!(run(source).ok().as_deref(), Some("loud!3\ntrue\ntrue\n"));
}

#[test]
fn lists_are_shared_and_nulls_fall_back() {
    let source = "class Main {\n const SIZE = 2 * 3\n static func main() {\n \
                  var list = [1, 2]\n const alias = list\n alias.push(SIZE)\n \
                  Debug.print(list.length().to_string())\n \
                  var maybe: [int]? = null\n Debug.print((maybe?.length() ?? -1).to_string())\n \
                  for value in list {\n list.clear()\n Debug.print(value.to_string())\n }\n \
                  Debug.print(list.is_empty().to_string())\n }\n}\n";
    // The loop goes through the elements the list had when it started.
    assert_eq!(run(source).ok().as_deref(), Some("3\n-1\n1\n2\n6\ntrue\n"));
}

#[test]
fn objects_are_built_in_two_phases() {
    let source = "class Box {\n var label: string\n var size = 1\n \
                  init(label: string) {\n var upper = label.to_upper()\n \
                  self.label = upper\n grow()\n Debug.print(upper + \" \" + size.to_string())\n }\n \
                  private func grow() {\n size *= 3\n }\n}\n\
                  class Main {\n static func main() {\n const box = Box(\"a\")\n \
                  box.size += 1\n Debug.print(box.label + box.size.to_string())\n }\n}\n";
    // `upper`, declared before the object exists, is still read after it.
    assert_eq!(run(source).ok().as_deref(), Some("A 3\nA4\n"));
}

#[test]
fn reading_past_the_end_is_a_fault() {
    let source = "class Main {\n static func main() {\n var list = [1]\n \
                  list[0] += 1\n Debug.print(list[0].to_string())\n list[1] = 0\n }\n}\n";
    let error = run_with_output(source);
    assert_eq!(error.0, "2\n");
    assert_eq!(
        error.1.map(|error| error.to_string()),
        Some("StarScript runtime error: index 1 is out of range for a list of length 1".to_owned())
    );
}

/// What `source` prints, and why it stopped if it did not end.
fn run_with_output(source: &str) -> (String, Option<RunError>) {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let (buffer, output) = buffer();
    let result = program.run(&output);
    (
        String::from_utf8(buffer.take()).expect("UTF-8 output"),
        result.err(),
    )
}

/// A buffer to print to, and the output that writes to it.
fn buffer() -> (Rc<RefCell<Vec<u8>>>, Output) {
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    (buffer, output)
}

#[test]
fn clones_share_the_program_and_runs_share_the_output() {
    let source = "class Main {\n static func main() {\n Debug.print(\"again\")\n }\n}\n";
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let copy = program.clone();
    let (buffer, output) = buffer();
    program.run(&output).expect("a run without errors");
    copy.run(&output).expect("a run without errors");
    assert_eq!(buffer.take(), b"again\nagain\n");
}

#[test]
fn a_failing_coroutine_stops_the_program_and_the_others() {
    let source = "class Main {\n static async func fail_later() {\n await Time.next_frame()\n \
                  var zero = 0\n Debug.print((1 / zero).to_string())\n }\n \
                  static async func tick() {\n for i in 0..5 {\n \
                  Debug.print(\"tick \" + i.to_string())\n await Time.next_frame()\n }\n }\n \
                  static func main() {\n fail_later()\n tick()\n Debug.print(\"main ends\")\n }\n}\n";
    let (printed, error) = run_with_output(source);
    // `fail_later` goes on first in the next frame, and `tick` never again.
    assert_eq!(printed, "tick 0\nmain ends\n");
    assert_eq!(
        error.map(|error| error.to_string()),
        Some("StarScript runtime error: integer division by zero".to_owned())
    );
    assert_eq!(star_script_runtime::tasks::waiting(), 0);
}

#[test]
fn a_coroutine_failing_before_its_first_wait_stops_who_started_it() {
    let source = "class Main {\n static async func fail() {\n var list = [1]\n \
                  Debug.print(list[3].to_string())\n }\n \
                  static func main() {\n fail()\n Debug.print(\"never\")\n }\n}\n";
    let (printed, error) = run_with_output(source);
    assert_eq!(printed, "");
    assert_eq!(
        error.map(|error| error.to_string()),
        Some("StarScript runtime error: index 3 is out of range for a list of length 1".to_owned())
    );
}

#[test]
fn awaits_wait_inside_loops_assignments_and_operators() {
    let source = "class Main {\n static async func three() -> int {\n \
                  await Time.next_frame()\n return 3\n }\n \
                  static async func main() {\n var total = 0\n var list = [1, 2]\n \
                  while total < await three() {\n total += 1\n }\n \
                  list[0] += await three()\n \
                  const lazy = true || await three() > 0\n \
                  Debug.print(total.to_string() + \" \" + list[0].to_string() + \" \" + lazy.to_string())\n }\n}\n";
    assert_eq!(run(source).ok().as_deref(), Some("3 4 true\n"));
}

#[test]
fn the_lowered_program_can_be_shared_between_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<crate::tree::Code>();
}

#[test]
fn literals_give_their_values() {
    let source = "class Main {\n static func main() {\n var text: string\n Debug.print(\"[\" + text + \"]\")\n Debug.print((\"a\" + \"b\").to_upper())\n Debug.print((2.5).to_string())\n }\n}\n";
    assert_eq!(run(source).expect("a program that runs"), "[]\nAB\n2.5\n");
}

#[test]
fn parallel_functions_fail_through_the_functions_they_call() {
    let source = "class Work {\n const LIMIT = 2\n parallel static func at(values: [int], index: int) -> int {\n return values[index]\n }\n parallel static func last(values: [[int]]) -> int {\n return at(values[0], LIMIT)\n }\n}\nclass Main {\n static async func main() {\n Debug.print((await Work.at([4, 5], 1)).to_string())\n Debug.print((await Work.last([[1, 2]])).to_string())\n Debug.print(\"never printed\")\n }\n}\n";

    let (printed, error) = run_with_output(source);

    assert_eq!(printed, "5\n");
    let Some(RunError::Fault(error)) = error else {
        panic!("a runtime error, found {error:?}");
    };
    assert!(
        error.message().contains("out of range"),
        "{}",
        error.message()
    );
}

#[test]
fn engine_objects_keep_their_fields_and_classes() {
    let source = "class Eye extends Camera2D {\n var seen = 0\n init(name: string) {\n super.init(name)\n main = true\n }\n                   override func update(delta: float) {\n seen += 1\n x += delta\n }\n}\n\
                  class Main {\n static func main() {\n const eye = Eye(\"eye\")\n eye.update(0.5)\n                   const node: Node = eye\n Debug.print(node.x.to_string() + \" \" + eye.seen.to_string() + \" \" + eye.main.to_string())\n                   Debug.print((node is Eye).to_string() + \" \" + (Node(\"n\") is Camera2D).to_string())\n                   const colors = [Color(0.5, 0.5, 0.5, 1.0)]\n colors[0].a -= 0.25\n Debug.print(colors[0].a.to_string())\n }\n}\n";
    assert_eq!(
        run(source).expect("a program that runs"),
        "0.5 1 true\ntrue false\n0.75\n"
    );
}

/// The seconds of each frame of [`play`].
const DELTA: f32 = 0.5;

/// Plays the game `source` without a window, as `star_script_engine`
/// does, for at most `frames` frames of [`DELTA`] seconds: what it
/// printed, and how it ended.
fn play(source: &str, frames: usize) -> (String, Result<(), RunError>) {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    assert_eq!(program.kind(), ProgramKind::Game);
    let (buffer, output) = buffer();
    let game = program
        .new_game(&output)
        .expect("a game that can be created");
    Interpreted::start_game(&game);
    let mut nodes = Vec::new();
    for _ in 0..frames {
        if Interpreted::game(&game).is_quit_requested() {
            break;
        }
        tasks::frame_started(DELTA);
        game::run_frame::<Interpreted>(&game, DELTA, &mut nodes);
        tasks::resume();
    }
    let result = end_game();
    let printed = String::from_utf8(buffer.take()).expect("UTF-8 output");
    (printed, result)
}

#[test]
fn games_start_update_and_quit() {
    let source = "class Counter extends Game {\n var frames = 0\n \
                  override func start() {\n Debug.print(\"start \" + background.r.to_string())\n background = Color(0.5, 0.0, 0.0, 1.0)\n }\n \
                  override func update(delta: float) {\n frames += 1\n \
                  Debug.print(\"frame \" + frames.to_string() + \" \" + delta.to_string() + \" \" + background.r.to_string())\n \
                  if frames == 3 {\n quit()\n }\n }\n}\n";

    let (printed, result) = play(source, 10);

    assert_eq!(
        printed,
        "start 0.0\nframe 1 0.5 0.5\nframe 2 0.5 0.5\nframe 3 0.5 0.5\n"
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn nodes_of_the_game_run_and_main_cameras_are_seen_from() {
    let source = "class Eye extends Camera2D {\n init() {\n super.init(\"Eye\")\n main = true\n }\n \
                  override func start() {\n super.start()\n Debug.print(\"eye start\")\n }\n \
                  override func update(delta: float) {\n x += delta\n Debug.print(\"eye \" + x.to_string())\n }\n}\n\
                  class Level extends Game {\n init() {\n add_child(Eye())\n }\n \
                  override func update(delta: float) {\n Debug.print(\"camera \" + (camera?.name ?? \"none\"))\n \
                  if (camera?.x ?? 0.0) >= 1.0 {\n quit()\n }\n }\n}\n";

    let (printed, result) = play(source, 10);

    assert_eq!(
        printed,
        "camera none\neye start\neye 0.5\ncamera Eye\neye 1.0\ncamera Eye\neye 1.5\n"
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn runtime_errors_close_the_game_and_stop_its_code() {
    let source = "class Broken extends Game {\n var values = [1]\n var frames = 0\n \
                  override func update(delta: float) {\n frames += 1\n Debug.print(\"frame \" + frames.to_string())\n \
                  if frames == 2 {\n Debug.print(values[3].to_string())\n }\n }\n}\n";

    let (printed, result) = play(source, 10);

    assert_eq!(printed, "frame 1\nframe 2\n");
    let Err(RunError::Fault(error)) = result else {
        panic!("a runtime error, found {result:?}");
    };
    assert!(
        error.message().contains("out of range"),
        "{}",
        error.message()
    );
}

#[test]
fn errors_of_coroutines_close_the_game_too() {
    let source = "class Waiting extends Game {\n override func start() {\n fail_later()\n }\n \
                  async func fail_later() {\n await Time.next_frame()\n \
                  Debug.print(\"later\")\n Debug.print([1][2].to_string())\n }\n \
                  override func update(delta: float) {\n Debug.print(\"frame\")\n }\n}\n";

    let (printed, result) = play(source, 10);

    // `next_frame` before the first frame goes on in the second, as compiled:
    // both use the runtime's scheduler.
    assert_eq!(printed, "frame\nframe\nlater\n");
    assert!(matches!(result, Err(RunError::Fault(_))), "{result:?}");
}

#[test]
fn games_and_console_programs_are_run_their_own_way() {
    let game = "class Empty extends Game {\n}\n";
    let program = Program::lower(&check_source(game).expect("a valid program"))
        .expect("a program the interpreter runs");
    let (_, output) = buffer();
    assert!(matches!(
        program.run(&output),
        Err(RunError::WrongKind(ProgramKind::Game))
    ));

    let console = "class Main {\n static func main() {\n }\n}\n";
    let program = Program::lower(&check_source(console).expect("a valid program"))
        .expect("a program the interpreter runs");
    assert_eq!(program.kind(), ProgramKind::Console);
    assert!(matches!(
        program.new_game(&output),
        Err(RunError::WrongKind(ProgramKind::Console))
    ));
}

#[test]
fn games_need_no_arguments_and_one_entry_point() {
    let arguments = "class Sized extends Game {\n var size: int\n init(size: int) {\n self.size = size\n }\n}\n";
    assert_eq!(
        lowering_error(arguments),
        LowerErrorKind::GameNeedsArguments
    );
    let both = "class Played extends Game {\n}\nclass Main {\n static func main() {\n }\n}\n";
    assert_eq!(lowering_error(both), LowerErrorKind::SeveralEntryPoints);
}
