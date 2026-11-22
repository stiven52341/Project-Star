//! Glue between the platform event loop and the engine subsystems.

use std::time::{Duration, Instant};

use star_core::{EngineResult, Time};
use star_ecs::{Schedule, Stage, World};
use star_input::Input;
use star_platform::{InputEvent, Window, WindowEventHandler, WindowSize};
use star_render::{Color, Frame, Renderer, RendererConfig};
use star_scene::{TransformPropagation, draw_sprite_nodes};

use crate::EngineWorld as _;
use crate::game::{Game, GameContext};
use crate::game_assets::GameAssets;

/// Engine state while the game is running.
pub(crate) struct Runtime<G: Game> {
    game: G,
    renderer_config: RendererConfig,
    clear_color: Color,
    renderer: Option<Renderer>,
    frame: Frame,
    /// Holds the game's entities and the engine's resources: [`Time`] and [`Input`].
    world: World,
    schedule: Schedule,
    transforms: TransformPropagation,
    assets: GameAssets,
    last_frame_start: Option<Instant>,
}

impl<G: Game> Runtime<G> {
    pub(crate) fn new(
        game: G,
        renderer_config: RendererConfig,
        clear_color: Color,
        input: Input,
        schedule: Schedule,
        assets: GameAssets,
    ) -> Self {
        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(input);
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
        }
    }

    fn advance_time(&mut self) {
        let now = Instant::now();
        let delta = self
            .last_frame_start
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        self.last_frame_start = Some(now);
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
    /// update right after the first stage, and then the global transforms
    /// of the nodes, ready to draw.
    fn simulate(&mut self) -> EngineResult<()> {
        self.schedule.run_stage(Stage::First, &mut self.world)?;
        self.game.update(&mut self.world);
        for stage in [Stage::Update, Stage::Physics, Stage::Last] {
            self.schedule.run_stage(stage, &mut self.world)?;
        }
        self.transforms.run(&mut self.world)?;
        if let Some(mut input) = self.world.resource_mut::<Input>() {
            input.end_frame();
        }
        Ok(())
    }
}

impl<G: Game> WindowEventHandler for Runtime<G> {
    fn on_window_created(&mut self, window: Window) -> EngineResult<()> {
        let mut renderer = Renderer::new(&window, self.renderer_config)?;
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
        self.renderer = Some(renderer);
        Ok(())
    }

    fn on_resized(&mut self, size: WindowSize) {
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
        self.frame.begin(self.clear_color);
        draw_sprite_nodes(&self.world, &mut self.frame)?;
        self.game.draw(&self.world, &mut self.frame);
        match &mut self.renderer {
            Some(renderer) => renderer.render(&self.frame),
            None => Ok(()),
        }
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
    use star_math::Vec2;
    use star_scene::{GlobalTransform2D, Hierarchy, Transform2D};

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
            input,
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
