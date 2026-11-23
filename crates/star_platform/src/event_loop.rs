//! Main event loop: creates the window and forwards OS events to the engine.

use star_core::{EngineError, EngineResult};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use crate::gilrs_input::GamepadEvents;
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

    /// Called when the system takes the window's surface away: an Android
    /// app going to the background. Nothing is drawn until
    /// [`on_resumed`](Self::on_resumed). By default it is ignored.
    fn on_suspended(&mut self) {}

    /// Called when the window can be drawn into again after
    /// [`on_suspended`](Self::on_suspended). By default it is ignored.
    ///
    /// # Errors
    ///
    /// An error stops the event loop and is returned by [`run_event_loop`].
    fn on_resumed(&mut self) -> EngineResult<()> {
        Ok(())
    }

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
    let event_loop = new_event_loop()?;
    // Games redraw continuously instead of waiting for OS events.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut adapter = WinitAdapter {
        config,
        handler,
        window: None,
        suspended: false,
        gamepads: GamepadEvents::new(),
        error: None,
    };
    event_loop
        .run_app(&mut adapter)
        .map_err(|error| EngineError::Platform(error.to_string()))?;

    adapter.error.map_or(Ok(()), Err)
}

/// The OS event loop. On Android it runs in the app the system started the
/// game in (see [`android_main!`](crate::android_main)).
fn new_event_loop() -> EngineResult<EventLoop<()>> {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid as _;
        let app = crate::android::app().ok_or_else(|| EngineError::Initialization {
            subsystem: "event loop",
            reason: "the game was not started with `android_main!`".to_owned(),
        })?;
        builder.with_android_app(app);
    }
    builder
        .build()
        .map_err(|error| EngineError::Initialization {
            subsystem: "event loop",
            reason: error.to_string(),
        })
}

/// Translates `winit` callbacks into [`WindowEventHandler`] calls.
struct WinitAdapter<'handler, H> {
    config: WindowConfig,
    handler: &'handler mut H,
    window: Option<Window>,
    /// Whether the system took the window's surface away; no frames are
    /// asked for until it gives it back.
    suspended: bool,
    gamepads: Option<GamepadEvents>,
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

    fn poll_gamepads(&mut self) {
        if let Some(gamepads) = &mut self.gamepads {
            gamepads.poll(|event| self.handler.on_input(event));
        }
    }

    fn stop_with_error(&mut self, event_loop: &ActiveEventLoop, error: EngineError) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<H: WindowEventHandler> ApplicationHandler for WinitAdapter<'_, H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let resumed = if self.window.is_none() {
            self.open_window(event_loop)
        } else if self.suspended {
            self.handler.on_resumed()
        } else {
            Ok(())
        };
        self.suspended = false;
        if let Err(error) = resumed {
            self.stop_with_error(event_loop, error);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if self.window.is_some() && !self.suspended {
            self.suspended = true;
            self.handler.on_suspended();
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
        self.poll_gamepads();
        if let Some(window) = self.window.as_ref().filter(|_| !self.suspended) {
            window.request_redraw();
        }
    }
}
