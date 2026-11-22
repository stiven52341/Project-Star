//! Glue between the platform event loop and the engine subsystems.

use std::time::{Duration, Instant};

use star_core::{EngineResult, Time};
use star_input::Input;
use star_platform::{InputEvent, Window, WindowEventHandler, WindowSize};
use star_render::{Color, Frame, Renderer, RendererConfig};

use crate::game::{Game, GameContext};

/// Engine state while the game is running.
pub(crate) struct Runtime<G: Game> {
    game: G,
    renderer_config: RendererConfig,
    clear_color: Color,
    renderer: Option<Renderer>,
    frame: Frame,
    time: Time,
    input: Input,
    last_frame_start: Option<Instant>,
}

impl<G: Game> Runtime<G> {
    pub(crate) fn new(game: G, renderer_config: RendererConfig, clear_color: Color) -> Self {
        Self {
            game,
            renderer_config,
            clear_color,
            renderer: None,
            frame: Frame::new(),
            time: Time::new(),
            input: Input::new(),
            last_frame_start: None,
        }
    }

    fn advance_time(&mut self) {
        let now = Instant::now();
        let delta = self
            .last_frame_start
            .map_or(Duration::ZERO, |last| now.duration_since(last));
        self.last_frame_start = Some(now);
        self.time.advance(delta);
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
        self.game.start(&mut GameContext::new(&mut renderer))?;
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
        self.game.update(&self.time, &self.input);
        self.input.end_frame();
        self.frame.begin(self.clear_color);
        self.game.draw(&mut self.frame);
        match &mut self.renderer {
            Some(renderer) => renderer.render(&self.frame),
            None => Ok(()),
        }
    }

    fn on_input(&mut self, event: InputEvent) {
        self.input.apply(event);
    }

    fn should_exit(&self) -> bool {
        self.game.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game that has already finished.
    struct FinishedGame;

    impl Game for FinishedGame {
        fn is_finished(&self) -> bool {
            true
        }
    }

    #[test]
    fn the_loop_stops_when_the_game_finishes() {
        let config = RendererConfig::default();

        assert!(Runtime::new(FinishedGame, config, Color::BLACK).should_exit());
        assert!(!Runtime::new(crate::game::EmptyGame, config, Color::BLACK).should_exit());
    }

    #[test]
    fn input_events_update_the_input_given_to_the_game() {
        let mut runtime = Runtime::new(
            crate::game::EmptyGame,
            RendererConfig::default(),
            Color::BLACK,
        );

        runtime.on_input(InputEvent::Key {
            key: star_input::Key::Space,
            state: star_input::ButtonState::Pressed,
        });

        assert!(runtime.input.keyboard().is_down(star_input::Key::Space));
    }
}
