//! Glue between the platform event loop and the engine subsystems.

use std::time::{Duration, Instant};

use star_audio::Audio;
use star_core::{EngineResult, Time};
use star_ecs::{Schedule, Stage, World};
use star_input::Input;
use star_math::Vec2;
use star_physics::{Physics2d, Physics3d};
use star_platform::Key;
use star_platform::{InputEvent, Window, WindowEventHandler, WindowSize};
use star_render::{Color, Frame, Renderer, RendererConfig, TextureFilter, TextureId};
use star_scene::{
    PhysicsNodes, PhysicsNodes3d, TransformPropagation, draw_3d_nodes, draw_sprite_nodes,
    step_node_physics, step_node_physics_3d,
};

use crate::EngineWorld as _;
use crate::FpsDisplay;
use crate::fps_counter::{FpsCounter, font_image};
use crate::game::{Game, GameContext};
use crate::game_assets::GameAssets;

/// Engine state while the game is running.
pub(crate) struct Runtime<G: Game> {
    game: G,
    renderer_config: RendererConfig,
    clear_color: Color,
    renderer: Option<Renderer>,
    frame: Frame,
    /// Holds the game's entities and the engine's resources (see
    /// [`engine_world`]).
    world: World,
    schedule: Schedule,
    transforms: TransformPropagation,
    assets: GameAssets,
    last_frame_start: Option<Instant>,
    fps: FpsCounter,
    /// The texture of the counter's font, once the window is open.
    fps_font: Option<TextureId>,
}

impl<G: Game> Runtime<G> {
    /// Runs `game` in `world`, made by [`engine_world`].
    pub(crate) fn new(
        game: G,
        renderer_config: RendererConfig,
        clear_color: Color,
        world: World,
        schedule: Schedule,
        assets: GameAssets,
    ) -> Self {
        Self {
            game,
            renderer_config,
            clear_color,
            renderer: None,
            frame: Frame::new(),
            world,
            schedule,
            transforms: TransformPropagation::new(),
            assets,
            last_frame_start: None,
            fps: FpsCounter::default(),
            fps_font: None,
        }
    }

    fn advance_time(&mut self) {
        let now = Instant::now();
        let delta = self
            .last_frame_start
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        self.last_frame_start = Some(now);
        self.fps.tick(delta.as_secs_f32());
        if let Some(mut time) = self.world.resource_mut::<Time>() {
            time.advance(delta);
        }
    }

    fn reload_changed_assets(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            let elapsed = self.world.time().delta();
            self.assets.reload_changed_textures(renderer, elapsed);
        }
    }

    /// One step of the simulation: the systems by stage, with the game's
    /// update right after the first stage and the physics of the nodes
    /// right before the physics stage, and then the global transforms of
    /// the nodes, ready to draw.
    fn simulate(&mut self) -> EngineResult<()> {
        self.schedule.run_stage(Stage::First, &mut self.world)?;
        self.game.update(&mut self.world);
        self.schedule.run_stage(Stage::Update, &mut self.world)?;
        if let Some(renderer) = &mut self.renderer {
            self.assets
                .give_collision_meshes(&mut self.world, renderer)?;
        }
        // Bodies are placed where their parents are now.
        self.transforms.run(&mut self.world)?;
        let elapsed = self.world.time().delta().as_secs_f32();
        step_node_physics(&self.world, elapsed)?;
        step_node_physics_3d(&self.world, elapsed)?;
        for stage in [Stage::Physics, Stage::Last] {
            self.schedule.run_stage(stage, &mut self.world)?;
        }
        self.transforms.run(&mut self.world)?;
        if let Some(mut input) = self.world.resource_mut::<Input>() {
            if input.keyboard().just_pressed(Key::F3)
                && let Some(mut display) = self.world.resource_mut::<FpsDisplay>()
            {
                display.shown = !display.shown;
            }
            input.end_frame();
        }
        Ok(())
    }
}

/// A world with the engine's resources: the [`Time`], the `input`, the
/// `audio`, the 2D physics ([`Physics2d`] with `gravity`, and the
/// [`PhysicsNodes`] of its bodies) and the 3D physics ([`Physics3d`] with
/// Earth's gravity, and the [`PhysicsNodes3d`] of its bodies). The
/// [`WindowSize`] comes when the window opens.
pub(crate) fn engine_world(input: Input, audio: Audio, gravity: Vec2) -> World {
    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(input);
    world.insert_resource(audio);
    world.insert_resource(Physics2d::new(gravity));
    world.insert_resource(PhysicsNodes::new());
    world.insert_resource(Physics3d::default());
    world.insert_resource(PhysicsNodes3d::new());
    world.insert_resource(FpsDisplay::default());
    world
}

impl<G: Game> WindowEventHandler for Runtime<G> {
    fn on_window_created(&mut self, window: Window) -> EngineResult<()> {
        let mut renderer = Renderer::new(&window, self.renderer_config)?;
        self.world.insert_resource(window.size());
        let gpu_info = renderer.gpu_info();
        log::info!("Rendering with {gpu_info}");
        if gpu_info.is_software {
            log::warn!("Rendering on the CPU: performance will be poor. Try another backend.");
        }
        self.game.start(&mut GameContext::new(
            &mut renderer,
            &mut self.world,
            &mut self.assets,
        ))?;
        self.fps_font = font_image()
            .and_then(|font| renderer.create_texture(&font, TextureFilter::Pixelated))
            .inspect_err(|error| log::warn!("The FPS counter cannot be shown: {error}"))
            .ok();
        self.renderer = Some(renderer);
        Ok(())
    }

    fn on_suspended(&mut self) {
        log::info!("The game is in the background: drawing stops");
        if let Some(renderer) = &mut self.renderer {
            renderer.suspend();
        }
    }

    fn on_resumed(&mut self) -> EngineResult<()> {
        // The time in the background is not a frame: the game goes on
        // where it stopped.
        self.last_frame_start = None;
        log::info!("The game is back: drawing again");
        self.renderer.as_mut().map_or(Ok(()), Renderer::resume)
    }

    fn on_resized(&mut self, size: WindowSize) {
        self.world.insert_resource(size);
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size);
        }
    }

    fn on_redraw_requested(&mut self) -> EngineResult<()> {
        if self.renderer.is_none() {
            return Ok(());
        }
        self.advance_time();
        self.reload_changed_assets();
        self.simulate()?;
        self.assets.play_sound_players(&self.world)?;
        self.frame.begin(self.clear_color);
        draw_sprite_nodes(&self.world, &mut self.frame)?;
        draw_3d_nodes(&self.world, &mut self.frame)?;
        let Some(renderer) = &mut self.renderer else {
            return Ok(());
        };
        self.assets
            .draw_sprite_files(&self.world, renderer, &mut self.frame)?;
        self.assets
            .draw_mesh_files(&self.world, renderer, &mut self.frame)?;
        self.assets
            .draw_models(&self.world, renderer, &mut self.frame)?;
        self.game.draw(&self.world, &mut self.frame);
        let shown = self
            .world
            .resource::<FpsDisplay>()
            .is_some_and(|display| display.shown);
        if let (true, Some(font)) = (shown, self.fps_font) {
            self.fps.draw(font, &mut self.frame);
        }
        renderer.render(&self.frame)
    }

    fn on_input(&mut self, event: InputEvent) {
        if let Some(mut input) = self.world.resource_mut::<Input>() {
            input.apply(event);
        }
    }

    fn should_exit(&self) -> bool {
        self.game.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use star_assets::AssetFolder;
    use star_core::EngineError;
    use star_input::{ActionMap, ButtonState, Key};
    use star_physics::{BodyKind, BodySettings, DEFAULT_GRAVITY, Shape2d, Shape3d};
    use star_scene::{
        Body2D, Body3D, GlobalTransform2D, GlobalTransform3D, Hierarchy, Transform2D, Transform3D,
    };

    use super::*;
    use crate::game::EmptyGame;

    /// A game that has already finished.
    struct FinishedGame;

    impl Game for FinishedGame {
        fn is_finished(&self) -> bool {
            true
        }
    }

    /// Who ran during a frame, in order.
    #[derive(Default)]
    struct Log(Vec<&'static str>);

    fn log_as(world: &World, name: &'static str) {
        if let Some(mut log) = world.resource_mut::<Log>() {
            log.0.push(name);
        }
    }

    /// A game that writes in the log when it updates.
    struct LoggingGame;

    impl Game for LoggingGame {
        fn update(&mut self, world: &mut World) {
            log_as(world, "game");
        }
    }

    fn runtime<G: Game>(game: G, input: Input, schedule: Schedule) -> Runtime<G> {
        Runtime::new(
            game,
            RendererConfig::default(),
            Color::BLACK,
            engine_world(input, Audio::silent(), DEFAULT_GRAVITY),
            schedule,
            GameAssets::new(AssetFolder::new("assets"), false),
        )
    }

    fn press_space<G: Game>(runtime: &mut Runtime<G>) {
        runtime.on_input(InputEvent::Key {
            key: Key::Space,
            state: ButtonState::Pressed,
        });
    }

    #[test]
    fn the_loop_stops_when_the_game_finishes() {
        let finished = runtime(FinishedGame, Input::new(), Schedule::new());
        let running = runtime(EmptyGame, Input::new(), Schedule::new());

        assert!(finished.should_exit());
        assert!(!running.should_exit());
    }

    #[test]
    fn the_world_holds_the_time_and_the_input() {
        let runtime = runtime(EmptyGame, Input::new(), Schedule::new());

        assert!(runtime.world.has_resource::<Time>());
        assert!(runtime.world.input().is_some());
    }

    #[test]
    fn f3_shows_and_hides_the_fps_counter() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());
        let shown =
            |runtime: &Runtime<EmptyGame>| runtime.world.resource::<FpsDisplay>().unwrap().shown;
        let press_f3 = |runtime: &mut Runtime<EmptyGame>, state| {
            runtime.on_input(InputEvent::Key {
                key: Key::F3,
                state,
            });
        };
        assert!(!shown(&runtime));

        press_f3(&mut runtime, ButtonState::Pressed);
        runtime.simulate().unwrap();
        assert!(shown(&runtime));
        runtime.simulate().unwrap();
        assert!(shown(&runtime), "held, it does not toggle again");

        press_f3(&mut runtime, ButtonState::Released);
        press_f3(&mut runtime, ButtonState::Pressed);
        runtime.simulate().unwrap();
        assert!(!shown(&runtime));
    }

    #[test]
    fn the_world_knows_the_window_size() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());

        runtime.on_resized(WindowSize::new(800, 600));

        assert_eq!(runtime.world.window_size(), WindowSize::new(800, 600));
    }

    #[test]
    fn input_events_update_the_input_in_the_world() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());

        press_space(&mut runtime);

        let input = runtime.world.input().unwrap();
        assert!(input.keyboard().is_down(Key::Space));
    }

    #[test]
    fn input_events_reach_the_game_actions() {
        let mut actions = ActionMap::new();
        actions.add("jump", &[Key::Space.into()]).unwrap();
        let mut runtime = runtime(EmptyGame, Input::with_actions(actions), Schedule::new());

        press_space(&mut runtime);

        let input = runtime.world.input().unwrap();
        assert!(input.actions().just_pressed("jump"));
    }

    #[test]
    fn the_game_updates_after_the_first_stage_and_before_the_rest() {
        let mut schedule = Schedule::new();
        for (stage, name) in [
            (Stage::Last, "last"),
            (Stage::Update, "update"),
            (Stage::First, "first"),
        ] {
            schedule.add_system(stage, move |world| {
                log_as(world, name);
                Ok(())
            });
        }
        let mut runtime = runtime(LoggingGame, Input::new(), schedule);
        runtime.world.insert_resource(Log::default());

        runtime.simulate().unwrap();

        let log = runtime.world.resource::<Log>().unwrap();
        assert_eq!(log.0, ["first", "game", "update", "last"]);
    }

    #[test]
    fn global_transforms_are_ready_after_the_systems_moved_the_nodes() {
        let mut schedule = Schedule::new();
        schedule.add_system(Stage::Last, |world| {
            world.each::<&mut Transform2D>(|_, transform| transform.position.x += 1.0)
        });
        let mut runtime = runtime(EmptyGame, Input::new(), schedule);
        let hero = runtime.world.spawn_node("Hero", None).unwrap();
        runtime.world.insert(hero, Transform2D::IDENTITY).unwrap();

        runtime.simulate().unwrap();

        let global = *runtime.world.get::<GlobalTransform2D>(hero).unwrap();
        assert_eq!(global.position(), Vec2::new(1.0, 0.0));
    }

    #[test]
    fn bodies_of_nodes_move_with_the_frames() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());
        let ball = runtime.world.spawn_node("Ball", None).unwrap();
        let settings = BodySettings::new(BodyKind::Dynamic, Shape2d::circle(0.5).unwrap());
        runtime.world.insert(ball, Transform2D::IDENTITY).unwrap();
        runtime.world.insert(ball, Body2D::new(settings)).unwrap();
        let mut time = Time::new();
        time.advance(Duration::from_millis(100));
        runtime.world.insert_resource(time);

        runtime.simulate().unwrap();

        assert!(runtime.world.get::<Body2D>(ball).unwrap().id().is_some());
        let global = *runtime.world.get::<GlobalTransform2D>(ball).unwrap();
        assert!(global.position().y < 0.0, "{global:?}");
    }

    #[test]
    fn bodies_of_3d_nodes_move_with_the_frames() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());
        let ball = runtime.world.spawn_node("Ball", None).unwrap();
        let settings = BodySettings::new(BodyKind::Dynamic, Shape3d::sphere(0.5).unwrap());
        runtime.world.insert(ball, Transform3D::IDENTITY).unwrap();
        runtime.world.insert(ball, Body3D::new(settings)).unwrap();
        let mut time = Time::new();
        time.advance(Duration::from_millis(100));
        runtime.world.insert_resource(time);

        runtime.simulate().unwrap();

        assert!(runtime.world.get::<Body3D>(ball).unwrap().id().is_some());
        let global = *runtime.world.get::<GlobalTransform3D>(ball).unwrap();
        assert!(global.position().y < 0.0, "{global:?}");
    }

    #[test]
    fn presses_last_one_frame() {
        let mut runtime = runtime(EmptyGame, Input::new(), Schedule::new());
        press_space(&mut runtime);

        runtime.simulate().unwrap();

        let input = runtime.world.input().unwrap();
        assert!(input.keyboard().is_down(Key::Space));
        assert!(!input.keyboard().just_pressed(Key::Space));
    }

    #[test]
    fn a_failing_system_stops_the_frame() {
        let mut schedule = Schedule::new();
        schedule.add_system(Stage::Update, |_| {
            Err(EngineError::Ecs("broken".to_owned()))
        });
        let mut runtime = runtime(EmptyGame, Input::new(), schedule);

        assert!(runtime.simulate().is_err());
    }
}
