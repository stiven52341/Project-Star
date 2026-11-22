//! Modules of the engine classes that scripts can extend, and the code that
//! runs a game in the engine.
//!
//! Their methods have no StarScript source, so their modules are written
//! here, with the same shape as the modules of script classes.

use super::conversions::RUNTIME;
use super::writer::Writer;
use crate::InputAction;

/// Hidden field of `Game` that `quit()` sets. It ends in a single `_`, so no
/// StarScript field can have the same Rust name.
const QUIT_FIELD: &str = "quit_requested_";

/// The module of the engine class `Game`.
pub(super) fn game_module(out: &mut Writer) {
    for line in GAME_MODULE.lines() {
        out.line(line);
    }
}

const GAME_MODULE: &str = "/// `class Game`, provided by the engine.
pub mod Game {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a game: whether it asked to close.
    pub struct Fields {
        pub quit_requested_: ::star_script_runtime::Field<bool>,
    }

    /// Gives access to the fields of this class, in objects of any subclass.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// What the engine calls: `start` once, `update` every frame.
    pub trait Object: crate::AnyObject + HasFields {
        fn start(self: ::std::rc::Rc<Self>);
        fn update(self: ::std::rc::Rc<Self>, delta: f32);
        fn quit(self: ::std::rc::Rc<Self>);
    }

    /// What games do unless they override it: nothing, until `quit`.
    pub struct Methods;

    impl Methods {
        /// `func start`
        pub fn start(this_: &Ref) {}

        /// `func update`
        pub fn update(this_: &Ref, delta: f32) {}

        /// `func quit`: closes the game after the current frame.
        pub fn quit(this_: &Ref) {
            HasFields::fields(&**this_).quit_requested_.set(true);
        }
    }

    /// An object created as this class (not as a subclass).
    pub struct Instance {
        pub Game: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Game }
    }

    impl crate::AnyObject for Instance {
        fn Game(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { Methods::start(&(self as Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { Methods::update(&(self as Ref), delta) }
        fn quit(self: ::std::rc::Rc<Self>) { Methods::quit(&(self as Ref)) }
    }

    /// Prepares an object of this class: `make_` builds it from its fields.
    pub fn init_<O: Object + 'static>(make_: impl FnOnce(Fields) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {
        make_(Fields { quit_requested_: ::star_script_runtime::Field::new(false) })
    }

    /// Creates an object of this class.
    pub fn new() -> Ref {
        init_(|fields0_| ::std::rc::Rc::new(Instance { Game: fields0_ }))
    }
}";

/// `fn main` that runs the game `class_path` in a window of the engine, with
/// the input `actions` of the project.
pub(super) fn game_entry_point(
    out: &mut Writer,
    class_path: &str,
    title: &str,
    actions: &[InputAction],
) {
    action_declarations(out, actions);
    let main = format!(
        "
/// Starts the program: runs the game in a window of the engine.
pub fn main() -> ::std::process::ExitCode {{
    match run_() {{
        ::std::result::Result::Ok(()) => ::std::process::ExitCode::SUCCESS,
        ::std::result::Result::Err(error_) => {{
            ::std::eprintln!(\"{{error_}}\");
            ::std::process::ExitCode::FAILURE
        }}
    }}
}}

fn run_() -> ::star_engine::star_core::EngineResult<()> {{
    let actions_ = action_map_()?;
    {RUNTIME}::input::declare_actions(&ACTION_NAMES_);
    let game_: crate::classes::Game::Ref = {class_path}::new();
    ::star_engine::app::App::new()
        .with_title({title:?})
        .with_actions(actions_)
        .run_game(EngineGame_(game_))
}}

/// Connects the StarScript game with the engine's game loop.
struct EngineGame_(crate::classes::Game::Ref);

impl ::star_engine::game::Game for EngineGame_ {{
    fn start(
        &mut self,
        _context: &mut ::star_engine::game::GameContext<'_>,
    ) -> ::star_engine::star_core::EngineResult<()> {{
        crate::classes::Game::Object::start(::std::rc::Rc::clone(&self.0));
        ::std::result::Result::Ok(())
    }}

    fn update(
        &mut self,
        time: &::star_engine::star_core::Time,
        input: &::star_engine::star_input::Input,
    ) {{
        share_actions_(input.actions());
        crate::classes::Game::Object::update(::std::rc::Rc::clone(&self.0), time.delta_seconds());
    }}

    fn is_finished(&self) -> bool {{
        crate::classes::Game::HasFields::fields(&*self.0).{QUIT_FIELD}.get()
    }}
}}

/// Copies the state of each action where `Input` reads it.
fn share_actions_(actions_: &::star_engine::star_input::Actions) {{
    for (index_, name_) in ACTION_NAMES_.iter().enumerate() {{
        let state_ = {RUNTIME}::input::ActionState {{
            is_down: actions_.is_down(name_),
            just_pressed: actions_.just_pressed(name_),
            just_released: actions_.just_released(name_),
        }};
        {RUNTIME}::input::set_action_state(index_, state_);
    }}
}}"
    );
    for line in main.lines() {
        out.line(line);
    }
}

/// The names of the actions and the function that builds their map. The
/// engine checks the bindings' names when the game starts.
fn action_declarations(out: &mut Writer, actions: &[InputAction]) {
    let names: Vec<String> = actions
        .iter()
        .map(|action| format!("{:?}", action.name))
        .collect();
    out.line("/// The input actions declared in the project.");
    out.line(&format!(
        "const ACTION_NAMES_: [&str; {}] = [{}];",
        actions.len(),
        names.join(", ")
    ));
    out.line("");
    out.open(
        "fn action_map_() -> ::star_engine::star_core::EngineResult<::star_engine::star_input::ActionMap> {",
    );
    out.line("let mut actions_ = ::star_engine::star_input::ActionMap::new();");
    for action in actions {
        let bindings: Vec<String> = action
            .bindings
            .iter()
            .map(|binding| format!("{binding:?}.parse()?"))
            .collect();
        out.line(&format!(
            "actions_.add({:?}, &[{}])?;",
            action.name,
            bindings.join(", ")
        ));
    }
    out.line("::std::result::Result::Ok(actions_)");
    out.close("}");
}
