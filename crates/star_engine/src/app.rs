//! Application builder: the starting point of every game.

use std::path::PathBuf;

use star_assets::AssetFolder;
use star_core::{EngineError, EngineResult};
use star_ecs::{Schedule, Stage, World};
use star_input::{ActionMap, Input};
use star_platform::{TargetPlatform, WindowConfig, WindowSize, run_event_loop};
use star_render::{Color, GraphicsBackend, RendererConfig};

use crate::game::{EmptyGame, Game};
use crate::game_assets::GameAssets;
use crate::runtime::Runtime;

/// A game application, configured with a builder and started with
/// [`App::run_game`] (or [`App::run`] for an empty window).
#[derive(Debug)]
pub struct App {
    window_config: WindowConfig,
    renderer_config: RendererConfig,
    clear_color: Color,
    actions: ActionMap,
    schedule: Schedule,
    assets: AssetFolder,
    hot_reload: bool,
    platform: TargetPlatform,
}

impl App {
    /// Creates an application with default settings for the current platform.
    #[must_use]
    pub fn new() -> Self {
        Self {
            window_config: WindowConfig::default(),
            renderer_config: RendererConfig::default(),
            clear_color: Color::BLACK,
            actions: ActionMap::new(),
            schedule: Schedule::new(),
            assets: AssetFolder::find_default(),
            hot_reload: cfg!(debug_assertions),
            platform: TargetPlatform::current(),
        }
    }

    /// Sets the window title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window_config.title = title.into();
        self
    }

    /// Sets the initial window size, in logical pixels.
    #[must_use]
    pub fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.window_config.size = WindowSize::new(width, height);
        self
    }

    /// Chooses the native graphics API (DirectX 12, Vulkan, Metal, OpenGL).
    #[must_use]
    pub fn with_graphics_backend(mut self, backend: GraphicsBackend) -> Self {
        self.renderer_config.backend = backend;
        self
    }

    /// Enables or disables vertical synchronization.
    #[must_use]
    pub fn with_vsync(mut self, enabled: bool) -> Self {
        self.renderer_config.vsync = enabled;
        self
    }

    /// Sets the color the screen is filled with before drawing each frame.
    #[must_use]
    pub fn with_clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    /// Sets the actions the game asks for, e.g. `"jump"`, and their bindings.
    #[must_use]
    pub fn with_actions(mut self, actions: ActionMap) -> Self {
        self.actions = actions;
        self
    }

    /// Runs `system` every frame in `stage`, after the systems added before.
    /// Systems see the world the game sees, with the engine's resources
    /// (see [`EngineWorld`](crate::EngineWorld)).
    #[must_use]
    pub fn with_system(
        mut self,
        stage: Stage,
        system: impl FnMut(&mut World) -> EngineResult<()> + 'static,
    ) -> Self {
        self.schedule.add_system(stage, system);
        self
    }

    /// Reads the game's assets from the folder at `root`, instead of the
    /// default one (see [`AssetFolder::find_default`]).
    #[must_use]
    pub fn with_asset_folder(mut self, root: impl Into<PathBuf>) -> Self {
        self.assets = AssetFolder::new(root);
        self
    }

    /// Enables or disables hot-reload: while the game runs, the textures
    /// loaded from files are updated when their files change. Enabled by
    /// default in debug builds, to see changes without restarting the game,
    /// and disabled in release builds, where assets do not change.
    #[must_use]
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }

    /// Whether asset files are reloaded when they change.
    #[must_use]
    pub fn hot_reload(&self) -> bool {
        self.hot_reload
    }

    /// The folder the game's assets are read from.
    #[must_use]
    pub fn asset_folder(&self) -> &AssetFolder {
        &self.assets
    }

    /// Window title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.window_config.title
    }

    /// Native graphics API requested for rendering.
    #[must_use]
    pub fn graphics_backend(&self) -> GraphicsBackend {
        self.renderer_config.backend
    }

    /// Platform the application is running on.
    #[must_use]
    pub fn platform(&self) -> TargetPlatform {
        self.platform
    }

    /// Opens a window that only shows the clear color, until it is closed.
    ///
    /// # Errors
    ///
    /// Same as [`App::run_game`].
    pub fn run(self) -> EngineResult<()> {
        self.run_game(EmptyGame)
    }

    /// Opens the window and runs `game` until the window is closed.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform is not supported, the window or the
    /// renderer cannot be created, the game fails to start, a system fails,
    /// or the GPU fails while rendering.
    pub fn run_game(self, game: impl Game) -> EngineResult<()> {
        if !self.platform.is_supported() {
            return Err(EngineError::UnsupportedPlatform(self.platform.to_string()));
        }
        let input = Input::with_actions(self.actions);
        let mut runtime = Runtime::new(
            game,
            self.renderer_config,
            self.clear_color,
            input,
            self.schedule,
            GameAssets::new(self.assets, self.hot_reload),
        );
        run_event_loop(self.window_config, &mut runtime)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_title() {
        let app = App::new().with_title("My Game");

        assert_eq!(app.title(), "My Game");
    }

    #[test]
    fn builder_sets_graphics_backend() {
        let app = App::new().with_graphics_backend(GraphicsBackend::Vulkan);

        assert_eq!(app.graphics_backend(), GraphicsBackend::Vulkan);
    }

    #[test]
    fn builder_adds_systems_to_their_stage() {
        let app = App::new()
            .with_system(Stage::Update, |_| Ok(()))
            .with_system(Stage::Last, |_| Ok(()));

        assert_eq!(app.schedule.system_count(Stage::Update), 1);
        assert_eq!(app.schedule.system_count(Stage::First), 0);
    }

    #[test]
    fn builder_sets_the_asset_folder() {
        let app = App::new().with_asset_folder("my_game/assets");

        assert_eq!(
            app.asset_folder().root(),
            std::path::Path::new("my_game/assets")
        );
    }

    #[test]
    fn builder_sets_hot_reload() {
        assert!(!App::new().with_hot_reload(false).hot_reload());
        assert!(App::new().with_hot_reload(true).hot_reload());
    }

    #[test]
    fn new_app_picks_backend_automatically() {
        assert_eq!(App::new().graphics_backend(), GraphicsBackend::Auto);
    }
}
