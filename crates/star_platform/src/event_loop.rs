//! Main event loop: creates the window and forwards OS events to the engine.

use star_core::{EngineError, EngineResult};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use crate::input_event::InputEvent;
use crate::window::{Window, WindowConfig, WindowSize};
use crate::winit_input::translate_window_event;

/// Receives the window events the engine cares about.
pub trait WindowEventHandler {
    /// Called once the window exists and can be drawn into.
    ///
    /// # Errors
    ///
    /// An error stops the event loop and is returned by [`run_event_loop`].
    fn on_window_created(&mut self, window: Window) -> EngineResult<()>;

    /// Called when the drawable area changes size.
    fn on_resized(&mut self, size: WindowSize);

    /// Called once per frame, when the window should be redrawn.
    ///
    /// # Errors
    ///
    /// An error stops the event loop and is returned by [`run_event_loop`].
    fn on_redraw_requested(&mut self) -> EngineResult<()>;

    /// Called for each input event, between frames. By default it is ignored.
    fn on_input(&mut self, _event: InputEvent) {}

    /// Whether the application wants to close, checked after each frame.
    /// By default it only closes when the user closes the window.
    fn should_exit(&self) -> bool {
        false
    }
}

/// Opens a window and runs the event loop until the window is closed.
///
/// # Errors
///
/// Returns an error if the window cannot be created, the OS event loop fails,
/// or the handler reports an error.
pub fn run_event_loop(
    config: WindowConfig,
    handler: &mut impl WindowEventHandler,
) -> EngineResult<()> {
    let event_loop = EventLoop::new().map_err(|error| EngineError::Initialization {
        subsystem: "event loop",
        reason: error.to_string(),
    })?;
    // Games redraw continuously instead of waiting for OS events.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut adapter = WinitAdapter {
        config,
        handler,
        window: None,
        error: None,
    };
    event_loop
        .run_app(&mut adapter)
        .map_err(|error| EngineError::Platform(error.to_string()))?;

    adapter.error.map_or(Ok(()), Err)
}

/// Translates `winit` callbacks into [`WindowEventHandler`] calls.
struct WinitAdapter<'handler, H> {
    config: WindowConfig,
    handler: &'handler mut H,
    window: Option<Window>,
    error: Option<EngineError>,
}

impl<H: WindowEventHandler> WinitAdapter<'_, H> {
    fn create_window(&self, event_loop: &ActiveEventLoop) -> EngineResult<Window> {
        let attributes = winit::window::Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(
                self.config.size.width,
                self.config.size.height,
            ));
        event_loop
            .create_window(attributes)
            .map(Window::new)
            .map_err(|error| EngineError::Initialization {
                subsystem: "window",
                reason: error.to_string(),
            })
    }

    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> EngineResult<()> {
        let window = self.create_window(event_loop)?;
        self.handler.on_window_created(window.clone())?;
        self.window = Some(window);
        Ok(())
    }

    fn stop_with_error(&mut self, event_loop: &ActiveEventLoop, error: EngineError) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<H: WindowEventHandler> ApplicationHandler for WinitAdapter<'_, H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.open_window(event_loop) {
            self.stop_with_error(event_loop, error);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !self.window.as_ref().is_some_and(|window| window.is(id)) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.handler
                    .on_resized(WindowSize::new(size.width, size.height));
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.handler.on_redraw_requested() {
                    self.stop_with_error(event_loop, error);
                }
            }
            other => {
                if let Some(input) = translate_window_event(&other) {
                    self.handler.on_input(input);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.handler.should_exit() {
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
