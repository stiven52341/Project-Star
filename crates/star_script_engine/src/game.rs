//! A StarScript game run by the engine's game loop.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use star_engine::EngineWorld as _;
use star_engine::app::App;
use star_engine::game::{Game, GameContext};
use star_engine::script_nodes::ScriptNodes;
use star_engine::star_audio::Audio;
use star_engine::star_core::EngineResult;
use star_engine::star_ecs::World;
use star_engine::star_render::{AmbientLight, Camera, Frame};
use star_script_runtime::nodes::Host;

use crate::input::{share_actions, share_pointer};
use crate::mirror::{engine_color, mirror_nodes, script_camera, script_camera_3d, sync_bodies};

/// Runs the game that `new_game` creates in `app`, whose input actions are
/// `actions` (in the order they were declared to the runtime). The game is
/// created once the runtime can load scenes and sounds, so its `init` can
/// load them too.
///
/// # Errors
///
/// The error creating the game, or the engine's running it.
pub fn run<H: Host>(
    app: App,
    actions: &'static [&'static str],
    new_game: impl FnOnce() -> EngineResult<H::Game>,
) -> EngineResult<()> {
    let audio = crate::connect_runtime(app.asset_folder());
    let game = new_game()?;
    app.run_game(EngineGame::<H> {
        audio,
        game,
        actions,
        nodes: Vec::new(),
        pending: Vec::new(),
        known: HashMap::new(),
        mirror: ScriptNodes::new(),
    })
}

/// Connects a StarScript game with the engine's game loop.
struct EngineGame<H: Host> {
    /// Where `Sound`s play: silent until the engine gives the game its audio.
    audio: Rc<RefCell<Audio>>,
    game: H::Game,
    actions: &'static [&'static str],
    /// Reused list of the nodes whose code runs in a frame.
    nodes: Vec<H::Node>,
    /// Reused list of the nodes still to mirror, with their parent's number.
    pending: Vec<(H::Node, Option<u64>)>,
    /// Reused map of the nodes of the tree by their number, to tell bodies
    /// what they touch.
    known: HashMap<u64, H::Node>,
    /// The node of the engine's world that mirrors each node of the game.
    mirror: ScriptNodes,
}

impl<H: Host> Game for EngineGame<H> {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        *self.audio.borrow_mut() = context.world().audio();
        H::start_game(&self.game);
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        if let Some(input) = world.input() {
            share_actions(&input, self.actions);
            share_pointer(&input, &self.view(world), world.window_size());
        }
        let delta = world.time().delta_seconds();
        let root = H::game(&self.game).root().clone();
        sync_bodies::<H>(
            world,
            &mut self.mirror,
            &root,
            &mut self.known,
            &mut self.nodes,
        );
        star_script_runtime::tasks::frame_started(delta);
        star_script_runtime::game::run_frame::<H>(&self.game, delta, &mut self.nodes);
        star_script_runtime::tasks::resume();
        mirror_nodes::<H>(world, &mut self.mirror, &root, &mut self.pending);
    }

    fn draw(&self, world: &World, frame: &mut Frame) {
        let fields = H::game(&self.game);
        frame.set_clear_color(fields.background.read(engine_color::<H>));
        let ambient = fields.ambient.read(engine_color::<H>);
        frame.set_ambient_light(AmbientLight::new(ambient, 1.0));
        let ambient_2d = fields.ambient_2d.read(engine_color::<H>);
        frame.set_ambient_light_2d(AmbientLight::new(ambient_2d, 1.0));
        frame.set_camera(self.view(world));
    }

    fn is_finished(&self) -> bool {
        H::game(&self.game).is_quit_requested()
    }
}

impl<H: Host> EngineGame<H> {
    /// The engine's camera that shows the game from its `camera_3d`, or
    /// else from its `camera`.
    fn view(&self, world: &World) -> Camera {
        let fields = H::game(&self.game);
        if let Some(camera) = fields.camera_3d.get() {
            return self
                .mirror
                .camera_3d(world, &script_camera_3d::<H>(&camera));
        }
        fields.camera.read(|camera| match camera {
            Some(camera) => self.mirror.camera(world, &script_camera::<H>(camera)),
            None => Camera::default(),
        })
    }
}
