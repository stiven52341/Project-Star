//! Tests of the shape of the generated code. Whether it compiles and runs
//! as expected is tested end to end in `tests/end_to_end.rs`.

use super::EntryPoint;
use crate::compile_source;

fn rust(source: &str) -> String {
    match compile_source(source) {
        Ok(program) => program.code,
        Err(errors) => {
            let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
            panic!("compile errors: {messages:#?}");
        }
    }
}

#[test]
fn each_class_gets_a_module() {
    let code = rust("class Player { var lives = 3 }\nclass Enemy {}");

    assert!(code.contains("pub mod Player {"));
    assert!(code.contains("pub mod Enemy {"));
    assert!(code.contains("pub lives: ::star_script_runtime::Field<i64>,"));
}

#[test]
fn rust_keywords_are_escaped() {
    let code = rust("class struct { var type = 1\n func fn(None: int) {} }");

    assert!(code.contains("pub mod r#struct {"));
    assert!(code.contains("pub r#type: "));
    assert!(code.contains("pub fn r#fn(this_: &Ref, None_: i64)"));
}

#[test]
fn subclasses_extend_the_trait_of_their_base() {
    let code = rust("class A { func f() {} }\nclass B extends A { override func f() {} }");

    assert!(code.contains("pub trait Object: crate::classes::A::Object + HasFields {"));
    assert!(code.contains("crate::classes::B::Methods::f(&(self as crate::classes::B::Ref))"));
}

#[test]
fn integers_wrap_around() {
    let code = rust("class A { func f(a: int) -> int { return a * 2 + 1 } }");

    assert!(code.contains("((a).wrapping_mul(2_i64)).wrapping_add(1_i64)"));
}

#[test]
fn entry_point_calls_static_main() {
    let code = rust("class Main { static func main() { Debug.print(\"hola\") } }");

    assert!(code.contains("pub fn main() {"));
    assert!(code.contains("crate::classes::Main::Methods::main();"));
}

#[test]
fn programs_without_main_have_no_entry_point() {
    assert!(!rust("class A {}").contains("pub fn main"));
}

#[test]
fn only_one_class_can_have_main() {
    let errors =
        compile_source("class A { static func main() {} }\nclass B { static func main() {} }")
            .expect_err("two entry points");

    assert!(
        errors[0]
            .to_string()
            .starts_with("a program starts in one place")
    );
}

#[test]
fn generated_code_has_no_license_notice() {
    // Generated code belongs to the game's author (CLAUDE.md 3.6).
    assert!(!rust("class A {}").contains("GPL"));
}

#[test]
fn games_run_in_the_engine() {
    let program = compile_source(
        "class Base extends Game {}\nclass MyGame extends Base { override func update(delta: float) { quit() } }",
    )
    .expect("valid program");

    assert_eq!(program.entry_point, EntryPoint::Game);
    assert!(program.uses_engine());
    assert!(program.code.contains("crate::classes::MyGame::new()"));
    assert!(program.code.contains(".with_title(\"MyGame\")"));
}

#[test]
fn console_programs_do_not_need_the_engine() {
    let program = compile_source("class A { static func main() {} }").expect("valid program");

    assert_eq!(program.entry_point, EntryPoint::Main);
    assert!(!program.uses_engine());
}

#[test]
fn a_program_has_one_entry_point() {
    let errors = compile_source("class G extends Game {}\nclass A { static func main() {} }")
        .expect_err("two entry points");

    assert!(
        errors[0]
            .to_string()
            .starts_with("a program starts in one place")
    );
}

#[test]
fn games_cannot_need_arguments() {
    let errors =
        compile_source("class G extends Game { var n: int\n init(n: int) { self.n = n } }")
            .expect_err("game with parameters");

    assert_eq!(
        errors[0].to_string(),
        "the game `G` must be creatable without arguments: its `init` cannot have parameters"
    );
}

fn game_with_jump_action() -> super::GeneratedProgram {
    let settings = crate::GameSettings {
        actions: vec![crate::InputAction {
            name: "jump".into(),
            bindings: vec!["Space".into(), "GamepadSouth".into()],
        }],
    };
    crate::compile_source_with(
        "class G extends Game {\n override func update(delta: float) {\n \
         if Input.just_pressed(\"jump\") { quit() }\n }\n}",
        &settings,
    )
    .expect("valid program")
}

#[test]
fn games_build_the_action_map_of_the_project() {
    let code = game_with_jump_action().code;

    assert!(code.contains("const ACTION_NAMES_: [&str; 1] = [\"jump\"];"));
    assert!(
        code.contains("actions_.add(\"jump\", &[\"Space\".parse()?, \"GamepadSouth\".parse()?])?;")
    );
    assert!(code.contains(".with_actions(actions_)"));
}

#[test]
fn input_calls_read_the_shared_action_state() {
    let code = game_with_jump_action().code;

    assert!(code.contains("::star_script_runtime::input::action_state(\"jump\").just_pressed"));
    assert!(code.contains("share_actions_(&input);"));
    assert!(code.contains("strength: input_.action_strength(name_),"));
    assert!(code.contains("world.input()"));
}

#[test]
fn axes_read_two_actions() {
    let code = rust(
        "class G extends Game {\n var left = \"left\"\n var right = \"right\"\n \
         override func update(delta: float) { Debug.print(Input.axis(left, right).to_string()) }\n}",
    );

    assert!(code.contains("::star_script_runtime::input::axis(&("));
}

#[test]
fn computed_action_names_are_borrowed() {
    let code = rust(
        "class G extends Game {\n var name = \"jump\"\n \
         override func update(delta: float) { if Input.is_down(name) { quit() } }\n}",
    );

    assert!(code.contains("::star_script_runtime::input::action_state(&("));
}

#[test]
fn games_without_actions_have_an_empty_map() {
    let code = rust("class G extends Game {}");

    assert!(code.contains("const ACTION_NAMES_: [&str; 0] = [];"));
}

#[test]
fn scripts_extend_the_engine_nodes() {
    let code = rust(
        "class Ship extends Sprite {\n init() { super.init(\"Ship\", \"ship.png\") }\n \
         override func update(delta: float) { x += delta }\n}",
    );

    assert!(code.contains("pub mod Node {"));
    assert!(code.contains("pub mod Sprite {"));
    assert!(code.contains("pub trait Object: crate::classes::Sprite::Object + HasFields {"));
    assert!(code.contains("crate::classes::Sprite::init_("));
    assert!(code.contains("pub Node: crate::classes::Node::Fields,"));
    assert!(code.contains("crate::classes::Node::HasFields::fields(&**this_).x"));
}

#[test]
fn games_run_the_code_of_their_nodes_and_mirror_them() {
    let code =
        rust("class G extends Game {\n override func start() { add_child(Node(\"Level\")) }\n}");

    assert!(code.contains("crate::classes::Node::process_tree_(&root_, delta_, &mut self.nodes);"));
    assert!(code.contains("mirror_nodes_(world, &mut self.mirror, &root_, &mut self.pending);"));
}

#[test]
fn games_show_the_colors_of_their_sprites_and_background() {
    let code = rust(
        "class G extends Game {\n override func start() {\n \
         background = Color(0.0, 0.0, 0.2, 1.0)\n }\n}",
    );

    assert!(code.contains("pub mod Color {"));
    assert!(code.contains("crate::classes::Color::new(0.0_f32, 0.0_f32, 0.2_f32, 1.0_f32)"));
    assert!(code.contains("color: look_.color.read(|color_| engine_color_(&**color_)),"));
    assert!(code.contains("frame.set_clear_color("));
}

#[test]
fn games_are_drawn_from_their_camera() {
    let code = rust(
        "class G extends Game {\n override func start() {\n \
         const eye = Camera(\"Eye\")\n add_child(eye)\n camera = eye\n }\n}",
    );

    assert!(code.contains("pub mod Camera {"));
    assert!(code.contains("self.mirror.camera(world, &script_camera_(&**camera_))"));
    assert!(code.contains("frame.set_camera(self.view_(world));"));
}

#[test]
fn games_share_the_pointer_seen_from_their_camera() {
    let code = rust(
        "class G extends Game {\n override func update(delta: float) {\n \
         if Input.has_pointer() { Debug.print(Input.pointer_x().to_string()) }\n }\n}",
    );

    assert!(code.contains("share_pointer_(&input, &self.view_(world), world.window_size());"));
    assert!(code.contains("::star_script_runtime::input::pointer().is_present"));
    assert!(code.contains("::star_script_runtime::input::pointer().x"));
}

#[test]
fn games_read_scenes_through_the_engine() {
    let code = rust(
        "class G extends Game {\n override func start() {\n \
         const level = Scene.load(\"one.scene\")\n }\n}",
    );

    assert!(code.contains("pub mod Scene {"));
    assert!(code.contains("crate::classes::Scene::load("));
    assert!(code.contains("scenes::set_loader(move |path_| read_scene_(&folder_, path_));"));
}

#[test]
fn games_move_bodies_with_the_engine() {
    let code = rust(
        "class G extends Game {\n override func start() {\n const ball = Node(\"Ball\")\n \
         ball.body = Body.circle(\"dynamic\", 0.5)\n ball.body?.apply_impulse(1.0, 0.0)\n \
         add_child(ball)\n }\n}",
    );

    assert!(code.contains("pub mod Body {"));
    assert!(code.contains("crate::classes::Body::circle("));
    assert!(code.contains("sync_bodies_(world, &mut self.mirror, &root_"));
    assert!(
        code.contains("body: match (node_.transform_3d.is_some(), node_.body_3d, node_.body) {")
    );
}

#[test]
fn games_mirror_their_3d_nodes() {
    let code = rust(
        "class G extends Game {\n override func start() {\n const eye = Camera3D(\"Eye\")\n \
         camera_3d = eye\n const crate_mesh = Mesh(\"Crate\", \"cube\")\n \
         crate_mesh.body = Body.box(\"static\", 1.0, 1.0, 1.0)\n crate_mesh.look_at(1.0, 0.0, 0.0)\n \
         add_child(crate_mesh)\n add_child(Light(\"Sun\", \"directional\"))\n }\n}",
    );

    for module in ["Node3D", "Mesh", "Light", "Camera3D"] {
        assert!(code.contains(&format!("pub mod {module} {{")), "{module}");
    }
    assert!(code.contains("crate::classes::Body::box_("));
    assert!(code.contains("with_mesh_(&node_, |mesh_| {"));
    assert!(code.contains("transform_3d: transform_3d_,"));
    assert!(code.contains("self.mirror.camera_3d(world, &script_camera_3d_(&*camera_))"));
    assert!(code.contains("frame.set_ambient_light("));
}

#[test]
fn games_play_sounds_through_the_engine() {
    let code = rust(
        "class G extends Game {\n var jump = Sound.load(\"jump.wav\")\n \
         override func start() {\n jump?.play()\n Audio.set_volume(0.5)\n }\n}",
    );

    assert!(code.contains("pub mod Sound {"));
    assert!(code.contains("crate::classes::Sound::load("));
    assert!(code.contains("::star_script_runtime::audio::set_volume(0.5_f32)"));
    assert!(code.contains("audio::set_backend(EngineAudio_ {"));
    assert!(code.contains("*self.audio.borrow_mut() = context.world().audio();"));
}

#[test]
fn scenes_create_the_node_classes_that_need_no_arguments() {
    let code = rust(
        "class Rock extends Sprite {\n init() { super.init(\"Rock\", \"rock.png\") }\n}\n\
         class Big extends Node {\n init(size: float) { super.init(\"Big\") }\n}\n\
         class Score {}",
    );

    assert!(code.contains("\"Rock\" => ::std::result::Result::Ok(crate::classes::Rock::new()),"));
    assert!(code.contains("its `init` needs arguments"));
    assert!(code.contains("`Score` is not a node: it does not extend `Node`"));
    assert!(code.contains("there is no class"));
}

#[test]
fn other_engine_classes_cannot_be_extended() {
    let errors = compile_source(
        "class Paint extends Color {\n init() { super.init(1.0, 1.0, 1.0, 1.0) }\n}",
    )
    .expect_err("engine base");
    assert_eq!(
        errors[0].to_string(),
        "classes cannot inherit from the engine class `Color` yet"
    );

    let errors = compile_source("class Loud extends Sound {}").expect_err("engine base");
    assert_eq!(
        errors[0].to_string(),
        "classes cannot inherit from the engine class `Sound` yet"
    );

    let errors = compile_source("class Loud extends Debug {}").expect_err("engine base");

    assert_eq!(
        errors[0].to_string(),
        "classes cannot inherit from the engine class `Debug` yet"
    );
}
