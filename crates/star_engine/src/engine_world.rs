//! Shortcuts to the resources the engine keeps in the world.

use std::cell::Ref;

use star_core::Time;
use star_ecs::World;
use star_input::Input;
use star_platform::WindowSize;

/// Quick access to the engine's resources in a [`World`]: the [`Time`] and
/// the [`Input`] of the current frame, and the [`WindowSize`]. The engine
/// adds them before the game starts and keeps them updated.
pub trait EngineWorld {
    /// The timing of the current frame (zero before the first one).
    fn time(&self) -> Time;

    /// What the player is doing. `None` only in a world the engine does not
    /// run, or while a system holds the input to change it.
    fn input(&self) -> Option<Ref<'_, Input>>;

    /// The size of the game's window, empty (zero) before it opens.
    fn window_size(&self) -> WindowSize;
}

impl EngineWorld for World {
    fn time(&self) -> Time {
        self.resource::<Time>()
            .map(|time| *time)
            .unwrap_or_default()
    }

    fn input(&self) -> Option<Ref<'_, Input>> {
        self.resource::<Input>()
    }

    fn window_size(&self) -> WindowSize {
        self.resource::<WindowSize>()
            .map_or(WindowSize::new(0, 0), |size| *size)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_engine_resources_are_read_from_the_world() {
        let mut world = World::new();
        let mut time = Time::new();
        time.advance(Duration::from_millis(16));
        world.insert_resource(time);
        world.insert_resource(Input::new());
        world.insert_resource(WindowSize::new(640, 480));

        assert_eq!(world.time().delta(), Duration::from_millis(16));
        assert!(world.input().is_some());
        assert_eq!(world.window_size(), WindowSize::new(640, 480));
    }

    #[test]
    fn worlds_without_them_have_no_time_input_or_window() {
        let world = World::new();

        assert_eq!(world.time().frame_count(), 0);
        assert!(world.input().is_none());
        assert!(world.window_size().is_empty());
    }
}
