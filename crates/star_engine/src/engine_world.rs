//! Shortcuts to the resources the engine keeps in the world.

use std::cell::Ref;

use star_core::Time;
use star_ecs::World;
use star_input::Input;

/// Quick access to the engine's resources in a [`World`]: the [`Time`] and
/// the [`Input`] of the current frame. The engine adds both before the game
/// starts and keeps them updated.
pub trait EngineWorld {
    /// The timing of the current frame (zero before the first one).
    fn time(&self) -> Time;

    /// What the player is doing. `None` only in a world the engine does not
    /// run, or while a system holds the input to change it.
    fn input(&self) -> Option<Ref<'_, Input>>;
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

        assert_eq!(world.time().delta(), Duration::from_millis(16));
        assert!(world.input().is_some());
    }

    #[test]
    fn worlds_without_them_have_no_time_or_input() {
        let world = World::new();

        assert_eq!(world.time().frame_count(), 0);
        assert!(world.input().is_none());
    }
}
